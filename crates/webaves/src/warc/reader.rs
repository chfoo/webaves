use std::io::{BufReader, Read, Take};

use crate::{
    compress::Decompressor,
    header::{HeaderMap, HeaderParser},
    io::{BufReadMoreExt, SourceCountRead},
};

use super::header::HeaderMapExt;
use super::WARCError;

/// Reads a WARC file.
///
/// Decompression is handled automatically by [Decompressor].
pub struct WARCReader<'a, S: Read> {
    stream: BufReader<Decompressor<'a, S>>,
    header_limit: u64,

    state: ReaderState,

    file_offset: u64,

    magic_bytes_buffer: Vec<u8>,
    header_buffer: Vec<u8>,

    record_id: String,
    block_file_offset: u64,
    block_length: u64,
    block_bytes_read: u64,
}

impl<'a, S: Read> WARCReader<'a, S> {
    /// Creates a `WARCReader` with the given input buffered stream.
    pub fn new(stream: S) -> Result<Self, WARCError> {
        Ok(Self {
            stream: BufReader::new(Decompressor::new_allow_unknown(stream)?),
            header_limit: 16_777_216,
            state: ReaderState::StartOfHeader,
            magic_bytes_buffer: Vec::new(),
            header_buffer: Vec::new(),
            file_offset: 0,
            record_id: String::new(),
            block_file_offset: 0,
            block_length: 0,
            block_bytes_read: 0,
        })
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> S {
        self.stream.into_inner().into_inner()
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
        assert!(self.state == ReaderState::StartOfHeader);

        let decompressor_stream = self.stream.get_ref();
        let start_file_offset = self.file_offset;
        let raw_file_offset = decompressor_stream.source_read_count();

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
            .read_limit_until(b'\n', &mut self.magic_bytes_buffer, self.header_limit)?;

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

        let amount = crate::header::read_until_boundary(
            &mut self.stream,
            &mut self.header_buffer,
            self.header_limit,
        )?;
        self.file_offset += amount;

        Ok(())
    }

    fn parse_header_lines(&mut self) -> Result<HeaderMap, WARCError> {
        tracing::debug!("parse_header_lines");

        match HeaderParser::new()
            .parse_header(crate::stringutil::trim_trailing_crlf(&self.header_buffer))
        {
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
        self.block_bytes_read = 0;

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
    pub fn read_block(&mut self) -> BlockReader<'a, '_, S> {
        assert!(self.state == ReaderState::EndOfHeader);
        tracing::debug!("read_block");

        let stream = self.stream.by_ref().take(self.block_length);
        self.state = ReaderState::InBlock;

        BlockReader {
            stream,
            num_bytes_read: &mut self.block_bytes_read,
        }
    }

    /// Finish reading a record.
    ///
    /// Panics when called out of sequence.
    pub fn end_record(&mut self) -> Result<MiscellaneousData, WARCError> {
        assert!(self.state == ReaderState::InBlock);
        tracing::debug!("end_record");

        self.file_offset += self.block_bytes_read;

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

        let mut stream = self.stream.by_ref().take(self.header_limit);

        self.header_buffer.clear();

        for _ in 0..2 {
            let buf_position = self.header_buffer.len();
            let amount =
                stream.read_limit_until(b'\n', &mut self.header_buffer, self.header_limit)?;
            self.file_offset += amount as u64;
            let line = &self.header_buffer[buf_position..];

            if line.is_empty() || !b"\r\n".contains(&line[0]) {
                return Err(WARCError::MalformedFooter {
                    offset: self.file_offset,
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReaderState {
    StartOfHeader,
    EndOfHeader,
    InBlock,
}

/// Reader stream for a record body.
pub struct BlockReader<'a, 'b, S: Read> {
    stream: Take<&'b mut BufReader<Decompressor<'a, S>>>,
    num_bytes_read: &'b mut u64,
}

impl<'a, 'b, S: Read> BlockReader<'a, 'b, S> {
    /// Number of bytes read in total from the (compressed) file.
    pub fn raw_file_offset(&self) -> u64 {
        self.stream.get_ref().get_ref().source_read_count()
    }
}

impl<'a, 'b, S: Read> Read for BlockReader<'a, 'b, S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.stream.read(buf)?;
        *self.num_bytes_read += size as u64;
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
