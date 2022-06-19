use std::io::{BufRead, BufReader, Read, Take};

use crate::{
    compress::Decompressor,
    header::{HeaderMap, HeaderParser},
};

use super::header::HeaderMapExt;
use super::WARCError;

/// Reads a WARC file.
///
/// Decompression is handled automatically by [Decompressor].
pub struct WARCReader<'a, S: Read> {
    stream: Option<BufReader<Decompressor<'a, S>>>,

    state: ReaderState,

    file_offset: u64,

    magic_bytes_buffer: Vec<u8>,
    line_buffer: Vec<u8>,
    header_buffer: Vec<u8>,

    record_id: String,
    block_file_offset: u64,
    block_length: u64,
}

impl<'a, S: Read> WARCReader<'a, S> {
    /// Creates a `WARCReader` with the given input buffered stream.
    pub fn new(stream: S) -> Result<Self, WARCError> {
        Ok(Self {
            stream: Some(BufReader::new(Decompressor::new_allow_unknown(stream)?)),
            state: ReaderState::StartOfHeader,
            magic_bytes_buffer: Vec::new(),
            line_buffer: Vec::new(),
            header_buffer: Vec::new(),
            file_offset: 0,
            record_id: String::new(),
            block_file_offset: 0,
            block_length: 0,
        })
    }

    /// Creates a `WARCReader` with the given input stream.
    pub fn new_read<R: Read>(reader: R) -> Result<WARCReader<'a, BufReader<R>>, WARCError> {
        WARCReader::new(BufReader::new(reader))
    }

    /// Starts reading a record and returns the header.
    ///
    /// The caller must call [Self::read_block] next to advance the stream.
    ///
    /// Panics when called out of sequence.
    ///
    /// Returns `None` when there are no more records in the stream.
    pub fn begin_record(&mut self) -> Result<Option<HeaderMetadata>, WARCError> {
        assert!(matches!(&self.state, ReaderState::StartOfHeader));

        let stream = self.stream.as_ref().unwrap().get_ref();
        let start_file_offset = self.file_offset;
        let raw_file_offset = stream.raw_input_read_count();

        tracing::debug!(
            file_offset = self.file_offset,
            raw_file_offset,
            "begin_record"
        );

        if !self.read_magic_bytes()? {
            return Ok(None);
        }
        self.read_header_lines()?;
        let header_map = self.parse_header_lines()?;
        self.prepare_for_block_read(&header_map)?;

        self.state = ReaderState::EndOfHeader;

        Ok(Some(HeaderMetadata {
            version: String::from_utf8_lossy(&self.magic_bytes_buffer)
                .trim()
                .to_string(),
            version_raw: &self.magic_bytes_buffer,
            header: header_map,
            header_raw: &self.header_buffer,
            block_length: self.block_length,
            file_offset: start_file_offset,
            raw_file_offset,
        }))
    }

    fn read_magic_bytes(&mut self) -> Result<bool, WARCError> {
        tracing::debug!("read_magic_bytes");

        self.magic_bytes_buffer.clear();
        self.stream
            .as_mut()
            .unwrap()
            .read_until(b'\n', &mut self.magic_bytes_buffer)?;

        self.file_offset += self.magic_bytes_buffer.len() as u64;

        tracing::trace!(magic_bytes_buffer = ?self.magic_bytes_buffer, "read_magic_bytes");

        if self.magic_bytes_buffer.is_empty() {
            return Ok(false);
        }

        if !(self.magic_bytes_buffer.starts_with(b"WARC/0.")
            || self.magic_bytes_buffer.starts_with(b"WARC/1."))
        {
            return Err(WARCError::UnknownFormat);
        }

        Ok(true)
    }

    fn read_header_lines(&mut self) -> Result<(), WARCError> {
        tracing::debug!("read_header_lines");

        self.header_buffer.clear();
        let stream = self.stream.as_mut().unwrap();

        loop {
            self.line_buffer.clear();
            stream.read_until(b'\n', &mut self.line_buffer)?;
            self.file_offset += self.line_buffer.len() as u64;

            if self.line_buffer.is_empty() || b"\r\n".contains(&self.line_buffer[0]) {
                break;
            }

            self.header_buffer.extend_from_slice(&self.line_buffer)
        }

        Ok(())
    }

    fn parse_header_lines(&mut self) -> Result<HeaderMap, WARCError> {
        tracing::debug!("parse_header_lines");

        match HeaderParser::new().parse_header(&self.header_buffer) {
            Ok(header_map) => Ok(header_map),
            Err(error) => Err(WARCError::MalformedHeader {
                offset: self.file_offset,
                source: Some(Box::new(error)),
            }),
        }
    }

    fn prepare_for_block_read(&mut self, header_map: &HeaderMap) -> Result<(), WARCError> {
        self.record_id = header_map
            .get_str("WARC-Record-ID")
            .unwrap_or_default()
            .to_string();
        self.block_file_offset = self.file_offset;
        self.block_length = header_map.get_parsed_required("Content-Length")?;

        tracing::debug!(
            block_file_offset = self.block_file_offset,
            block_length = self.block_length,
            "prepare_for_block_read"
        );

        Ok(())
    }

    /// Starts reading a record body.
    ///
    /// The caller must read until the block stream is empty and then
    /// call [Self::end_record].
    ///
    /// Panics when called out of sequence.
    pub fn read_block(&mut self) -> BlockReader<'a, S> {
        assert!(matches!(&self.state, ReaderState::EndOfHeader));
        tracing::debug!("read_block");

        let stream = self.stream.take().unwrap().take(self.block_length);
        self.state = ReaderState::InBlock;

        BlockReader {
            stream,
            num_bytes_read: 0,
        }
    }

    /// Finish reading a record.
    ///
    /// Panics when called out of sequence.
    pub fn end_record(
        &mut self,
        block_reader: BlockReader<'a, S>,
    ) -> Result<MiscellaneousData, WARCError> {
        assert!(matches!(&self.state, ReaderState::InBlock));
        tracing::debug!("end_record");
        assert!(self.stream.is_none());

        self.stream = Some(block_reader.stream.into_inner());
        self.file_offset += block_reader.num_bytes_read;

        self.check_block_length()?;
        self.read_end_of_record_lines()?;

        self.state = ReaderState::StartOfHeader;

        Ok(MiscellaneousData {
            raw: &self.header_buffer,
        })
    }

    fn check_block_length(&self) -> Result<(), WARCError> {
        let current_offset = self.file_offset;
        let expected_offset = self.block_file_offset + self.block_length;

        tracing::debug!(current_offset, expected_offset, "check_block_length");

        if current_offset != expected_offset {
            return Err(WARCError::WrongBlockLength {
                record_id: self.record_id.clone(),
            });
        }

        Ok(())
    }

    fn read_end_of_record_lines(&mut self) -> Result<(), WARCError> {
        tracing::debug!("read_end_of_record_lines");

        let stream = self.stream.as_mut().unwrap();

        self.header_buffer.clear();

        for _ in 0..2 {
            self.line_buffer.clear();
            stream.read_until(b'\n', &mut self.line_buffer)?;
            self.file_offset += self.line_buffer.len() as u64;
            self.header_buffer.extend_from_slice(&self.line_buffer);

            if self.line_buffer.is_empty() || !b"\r\n".contains(&self.line_buffer[0]) {
                return Err(WARCError::MalformedFooter {
                    offset: self.file_offset,
                });
            }
        }

        Ok(())
    }
}

enum ReaderState {
    StartOfHeader,
    EndOfHeader,
    InBlock,
}

/// Reader stream for a record body.
pub struct BlockReader<'a, S: Read> {
    stream: Take<BufReader<Decompressor<'a, S>>>,
    num_bytes_read: u64,
}

impl<'a, S: Read> BlockReader<'a, S> {
    /// Number of bytes read in total from the (compressed) file.
    pub fn raw_file_offset(&self) -> u64 {
        self.stream.get_ref().get_ref().raw_input_read_count()
    }
}

impl<'a, S: Read> Read for BlockReader<'a, S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.stream.read(buf)?;
        self.num_bytes_read += size as u64;
        Ok(size)
    }
}

/// Noncritical data.
pub struct MiscellaneousData<'a> {
    raw: &'a [u8],
}

impl<'a> MiscellaneousData<'a> {
    /// Returns the raw bytes.
    pub fn raw(&self) -> &[u8] {
        self.raw
    }
}

/// A record's header and associated file metadata.
pub struct HeaderMetadata<'a> {
    version: String,
    version_raw: &'a [u8],
    header: HeaderMap,
    header_raw: &'a [u8],
    block_length: u64,
    file_offset: u64,
    raw_file_offset: u64,
}

impl<'a> HeaderMetadata<'a> {
    /// Returns the WARC record version.
    pub fn version(&self) -> &str {
        self.version.as_ref()
    }

    /// Returns the raw bytes of WARC record version.
    pub fn version_raw(&self) -> &[u8] {
        self.version_raw
    }

    /// Returns the parsed name-value fields.
    pub fn header(&self) -> &HeaderMap {
        &self.header
    }

    /// Returns the raw bytes of the name-value fields.
    pub fn header_raw(&self) -> &[u8] {
        self.header_raw
    }

    /// Returns the length of the body of the record.
    pub fn block_length(&self) -> u64 {
        self.block_length
    }

    /// Number of bytes read in total from the (uncompressed) stream.
    pub fn file_offset(&self) -> u64 {
        self.file_offset
    }

    /// Number of bytes read in total from the (compressed) stream.
    pub fn raw_file_offset(&self) -> u64 {
        self.raw_file_offset
    }
}
