use std::io::Write;

use crate::{
    compress::{CompressionFormat, CompressionLevel, Compressor},
    header::{HeaderFormatter, HeaderMap},
    warc::HeaderMapExt,
};

use super::WARCError;

/// Default WARC version string.
pub const DEFAULT_VERSION: &str = "WARC/1.1";

/// Writes a WARC file.
///
/// The writer handles compression automatically as multi-streams for rapid
/// access to records by consuming software.
/// If a stream with compression is given to this writer (although allowed
/// by the WARC format), the output WARC file will not indexable and seekable.
pub struct WARCWriter<'a, S: Write> {
    stream: Option<S>,

    state: WriterState,

    compressed_stream: Option<Compressor<'a, S>>,
    compression_format: CompressionFormat,
    compression_level: CompressionLevel,

    version: String,
    header_formatter: HeaderFormatter,

    record_id: String,
    block_length: u64,
    block_amount_written: u64,
}

impl<'a, S: Write> WARCWriter<'a, S> {
    /// Creates a writer with the given stream without compression.
    pub fn new(stream: S) -> Self {
        Self::new_compressed(stream, CompressionFormat::Raw, Default::default())
    }

    /// Creates a writer with the given stream and compression configuration.
    pub fn new_compressed(
        stream: S,
        compression_format: CompressionFormat,
        compression_level: CompressionLevel,
    ) -> Self {
        Self {
            stream: Some(stream),
            state: WriterState::StartOfHeader,
            compressed_stream: None,
            compression_format,
            compression_level,
            version: DEFAULT_VERSION.to_string(),
            header_formatter: HeaderFormatter::new(),
            record_id: String::new(),
            block_length: 0,
            block_amount_written: 0,
        }
    }

    /// Returns the formatter for headers.
    pub fn header_formatter(&self) -> &HeaderFormatter {
        &self.header_formatter
    }
    /// Sets the formatter for headers.
    pub fn set_header_formatter(&mut self, header_formatter: HeaderFormatter) {
        self.header_formatter = header_formatter;
    }

    /// Returns the WARC version string used when writing headers.
    ///
    /// Default: [DEFAULT_VERSION]
    pub fn version(&self) -> &str {
        self.version.as_ref()
    }

    /// Sets the WARC version string used when writing headers.
    pub fn set_version(&mut self, version: String) {
        self.version = version;
    }

    /// Returns the wrapped stream.
    ///
    /// Panics if the writer is in the middle of writing a record.
    pub fn into_inner(self) -> S {
        self.stream.unwrap()
    }

    /// Begins a record by writing the header.
    ///
    /// Writes the WARC version, the header as name-value fields, and
    /// the ending newline.
    ///
    /// The caller must call [Self::write_block] next to advance the stream.
    ///
    /// Panics when called out of sequence.
    pub fn begin_record(&mut self, header: &HeaderMap) -> Result<(), WARCError> {
        assert!(self.state == WriterState::StartOfHeader);
        assert!(self.stream.is_some());
        assert!(self.compressed_stream.is_none());

        tracing::debug!("begin_record");

        self.create_compressor()?;
        self.write_header(header)?;
        self.prepare_for_block_write(header)?;

        self.state = WriterState::EndOfHeader;

        Ok(())
    }

    fn create_compressor(&mut self) -> Result<(), WARCError> {
        tracing::debug!("create_compressor");

        let stream = self.stream.take().unwrap();
        let stream = Compressor::new(stream, self.compression_format, self.compression_level)?;
        self.compressed_stream = Some(stream);

        Ok(())
    }

    fn write_header(&mut self, header: &HeaderMap) -> Result<(), WARCError> {
        tracing::debug!("write_header");

        let mut stream = self.compressed_stream.as_mut().unwrap();

        stream.write_all(self.version.as_bytes())?;
        stream.write_all(b"\r\n")?;
        if let Err(error) = self.header_formatter.format_header(header, &mut stream) {
            return Err(WARCError::MalformedHeader {
                offset: 0,
                source: Some(Box::new(error)),
            });
        }
        stream.write_all(b"\r\n")?;

        Ok(())
    }

    fn prepare_for_block_write(&mut self, header: &HeaderMap) -> Result<(), WARCError> {
        self.record_id = header
            .get_str("WARC-Record-Id")
            .unwrap_or_default()
            .to_string();
        self.block_length = header.get_parsed_required("Content-Length")?;
        self.block_amount_written = 0;

        tracing::debug!(block_length = self.block_length, "prepare_for_block_write");

        Ok(())
    }

    /// Starts writing a record body.
    ///
    /// The caller must write all the block contents and then call [Self::end_record].
    /// The amount of bytes written must match `Content-Length` in the name-value fields.
    ///
    /// Panics when called out of sequence.
    pub fn write_block(&mut self) -> BlockWriter<'a, '_, S> {
        assert!(self.state == WriterState::EndOfHeader);
        tracing::debug!("write_block");

        self.state = WriterState::InBlock;

        BlockWriter {
            stream: self.compressed_stream.as_mut().unwrap(),
            num_bytes_written: &mut self.block_amount_written,
        }
    }

    /// Finish writing a record.
    ///
    /// Panics when called out of sequence.
    pub fn end_record(&mut self) -> Result<(), WARCError> {
        assert!(self.state == WriterState::InBlock);
        tracing::debug!("end_record");
        assert!(self.stream.is_none());
        assert!(self.compressed_stream.is_some());

        self.check_block_length()?;

        let mut stream = self.compressed_stream.take().unwrap();
        stream.write_all(b"\r\n\r\n")?;
        let mut stream = stream.finish()?;
        stream.flush()?;
        self.stream = Some(stream);
        self.state = WriterState::StartOfHeader;

        Ok(())
    }

    fn check_block_length(&self) -> Result<(), WARCError> {
        tracing::debug!(
            bytes_written = self.block_amount_written,
            block_length = self.block_length,
            "check_block_length"
        );

        if self.block_amount_written != self.block_length {
            return Err(WARCError::WrongBlockLength {
                record_id: self.record_id.clone(),
            });
        }

        Ok(())
    }
}

/// Writer stream for a record body.
pub struct BlockWriter<'a, 'b, S: Write> {
    stream: &'b mut Compressor<'a, S>,
    num_bytes_written: &'b mut u64,
}

impl<'a, 'b, S: Write> Write for BlockWriter<'a, 'b, S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let amount = self.stream.write(buf)?;
        *self.num_bytes_written += amount as u64;
        Ok(amount)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriterState {
    StartOfHeader,
    EndOfHeader,
    InBlock,
}
