use std::{
    io::{BufRead, Read, Take},
    str::FromStr,
};

use crate::{
    compress::{CompressionFormat, Decompressor},
    header::HeaderMap,
    io::PeekRead,
};

use super::{
    chunked::ChunkedReader, field::HeaderMapExt, ChunkedEncodingOption, CompressionOption,
    HTTPError, RequestHeader, ResponseHeader,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReaderState {
    Header,
    Body,
}

/// HTTP request and response reader.
pub struct MessageReader<'a, R: BufRead + PeekRead> {
    stream: Option<R>,
    body_reader: Option<BodyReader<'a, R>>,
    chunked_encoding: ChunkedEncodingOption,
    compression: CompressionOption,
    header_limit: u64,
    state: ReaderState,
    buffer: Vec<u8>,
    content_length: Option<u64>,
}

impl<'a, R: BufRead + PeekRead> MessageReader<'a, R> {
    /// Creates a new MessageReader with the given stream.
    pub fn new(stream: R) -> Self {
        Self {
            stream: Some(stream),
            body_reader: None,
            chunked_encoding: Default::default(),
            compression: Default::default(),
            header_limit: 65536,
            state: ReaderState::Header,
            buffer: Vec::new(),
            content_length: None,
        }
    }

    /// Returns the chunked transfer coding option.
    pub fn chunked_encoding(&self) -> ChunkedEncodingOption {
        self.chunked_encoding
    }

    /// Sets the chunked transfer coding option.
    pub fn set_chunked_encoding(&mut self, chunked_encoding: ChunkedEncodingOption) {
        self.chunked_encoding = chunked_encoding;
    }

    /// Returns the compression option for content-encoding/transfer-encoding.
    pub fn compression(&self) -> CompressionOption {
        self.compression
    }

    /// Sets the compression option for content-encoding/transfer-encoding.
    ///
    /// Only one compression method is supported.
    pub fn set_compression(&mut self, compression: CompressionOption) {
        self.compression = compression;
    }

    /// Begins reading a HTTP request and returns the header.
    ///
    /// [Self::read_body] must be called next to advance stream.
    ///
    /// Panics when called out of sequence.
    pub fn begin_request(&mut self) -> Result<RequestHeader, HTTPError> {
        tracing::debug!("begin_request");
        assert!(self.state == ReaderState::Header);
        self.read_header()?;

        let header =
            RequestHeader::parse_from(crate::stringutil::trim_trailing_crlf(&self.buffer))?;

        self.set_up_request_body(&header)?;
        self.state = ReaderState::Body;

        Ok(header)
    }

    /// Begins reading a HTTP response and returns the header.
    ///
    /// For additional validation, supply the request header to `initiator`.
    ///
    /// [Self::read_body] must be called next to advance stream.
    ///
    /// Panics when called out of sequence.
    pub fn begin_response(
        &mut self,
        initiator: Option<&RequestHeader>,
    ) -> Result<ResponseHeader, HTTPError> {
        tracing::debug!("begin_response");
        assert!(self.state == ReaderState::Header);

        let header = if self.check_http_response_magic_bytes()? {
            self.read_header()?;
            ResponseHeader::parse_from(crate::stringutil::trim_trailing_crlf(&self.buffer))?
        } else {
            tracing::debug!("using HTTP/0.9");
            ResponseHeader::new_09()
        };

        self.set_up_response_body(&header, initiator)?;
        self.state = ReaderState::Body;

        Ok(header)
    }

    fn read_header(&mut self) -> Result<(), HTTPError> {
        tracing::debug!("read_header");

        let stream = self.stream.as_mut().unwrap();

        self.buffer.clear();
        crate::header::read_until_boundary(stream, &mut self.buffer, self.header_limit)?;

        Ok(())
    }

    fn check_http_response_magic_bytes(&mut self) -> Result<bool, HTTPError> {
        tracing::trace!("check_http_response_magic_bytes");

        let stream = self.stream.as_mut().unwrap();
        let mut buffer = [0u8; 5];

        match stream.peek_exact(5) {
            Ok(data) => {
                tracing::debug!(?data, "check_http_response_magic_bytes");

                buffer.copy_from_slice(data);
                buffer.make_ascii_lowercase();

                Ok(buffer.starts_with(b"http/"))
            }
            Err(error) => {
                tracing::debug!(?error, "check_http_response_magic_bytes");
                Ok(false)
            }
        }
    }

    fn set_up_request_body(&mut self, header: &RequestHeader) -> Result<(), HTTPError> {
        self.content_length = self.parse_content_length(&header.fields, None, None)?;

        tracing::debug!(content_length = self.content_length, "set_up_request_body");

        self.set_up_body_common(&header.fields)?;

        Ok(())
    }

    fn set_up_response_body(
        &mut self,
        header: &ResponseHeader,
        initiator: Option<&RequestHeader>,
    ) -> Result<(), HTTPError> {
        self.content_length = self.parse_content_length(&header.fields, initiator, Some(header))?;

        tracing::debug!(content_length = self.content_length, "set_up_response_body");

        self.set_up_body_common(&header.fields)?;

        Ok(())
    }

    fn set_up_body_common(&mut self, fields: &HeaderMap) -> Result<(), HTTPError> {
        let stream = self.stream.take().unwrap();

        let is_chunked = self.is_chunked(fields);
        let layer = if is_chunked {
            BodyTransportLayer::Chunked(ChunkedReader::new(stream))
        } else {
            match self.content_length {
                Some(content_length) => BodyTransportLayer::Length(ExpectedLengthReader {
                    stream: stream.take(content_length),
                    current_length: 0,
                    expected_length: content_length,
                }),
                None => BodyTransportLayer::Legacy(stream),
            }
        };

        let compression_format = self.get_compression_format(fields);
        let decompressor = Decompressor::new_format(layer, compression_format)?;

        tracing::debug!(is_chunked, ?compression_format, "set_up_body_common");

        self.body_reader = Some(BodyReader {
            stream: decompressor,
        });

        Ok(())
    }

    fn is_chunked(&self, fields: &HeaderMap) -> bool {
        // RFC 9112 6.1: when transfer-encoding is supplied, the sender
        // must always use chunked as the last encoding. So we don't explicitly
        // checked for "chunked".
        match self.chunked_encoding {
            ChunkedEncodingOption::Off => false,
            ChunkedEncodingOption::On => true,
            ChunkedEncodingOption::Auto => !fields
                .get_str("transfer-encoding")
                .unwrap_or_default()
                .is_empty(),
        }
    }

    fn get_compression_format(&self, fields: &HeaderMap) -> CompressionFormat {
        match self.compression {
            CompressionOption::None => CompressionFormat::Raw,
            CompressionOption::Auto => self.get_compression_format_from_headers(fields),
            CompressionOption::Manual(format) => format,
        }
    }

    fn get_compression_format_from_headers(&self, fields: &HeaderMap) -> CompressionFormat {
        // We assume that if compression is specified in transfer-encoding, then
        // only one compression coding is specified and no content-encoding is
        // specified.
        // We assume that is content-encoding is specified, no compression
        // is specified in transfer-encoding and only one compression coding
        // is specified in content-encoding.

        let mut field_values = fields.get_comma_list("transfer-encoding");
        field_values.extend_from_slice(&fields.get_comma_list("content-encoding"));
        field_values.retain(|name| name != "identity" && name != "chunked");

        if field_values.len() > 1 {
            tracing::warn!(codings = ?field_values, "multiple content coding");
        }

        for format_name in field_values {
            if let Ok(format) = CompressionFormat::from_str(&format_name) {
                return format;
            }
        }

        CompressionFormat::Raw
    }

    fn parse_content_length(
        &self,
        fields: &HeaderMap,
        request: Option<&RequestHeader>,
        response: Option<&ResponseHeader>,
    ) -> Result<Option<u64>, HTTPError> {
        // RFC 9112 6.2 and 6.3
        if let Some(request) = request {
            if request.request_line.method == "HEAD" {
                tracing::trace!("parse_content_length head");
                return Ok(Some(0));
            }
        }

        if let Some(response) = response {
            if response.status_line.status_code >= 100 && response.status_line.status_code < 200
                || response.status_line.status_code == 204
                || response.status_line.status_code == 304
            {
                tracing::trace!("parse_content_length status_code");
                return Ok(Some(0));
            }
        }

        if fields.contains_key("transfer-encoding") {
            tracing::trace!("parse_content_length transfer-encoding");

            return Ok(None);
        }

        let lengths = fields.get_comma_list("content-length");

        if !lengths.is_empty() {
            if lengths.iter().all(|item| item == &lengths[0]) {
                let length =
                    lengths[0]
                        .parse::<u64>()
                        .map_err(|error| HTTPError::MalformedHeader {
                            source: Some(Box::new(error)),
                        })?;
                tracing::trace!("parse_content_length content-length");

                return Ok(Some(length));
            } else {
                return Err(HTTPError::MalformedHeader { source: None });
            }
        }

        if response.is_some() {
            tracing::trace!("parse_content_length response");

            Ok(None)
        } else {
            tracing::trace!("parse_content_length request");

            Ok(Some(0))
        }
    }

    /// Returns a reader for reading the message body.
    ///
    /// Once the message body reaches EOF, [Self::end_message] must be called
    /// to finished reading the body.
    ///
    /// Panics when called out of sequence.
    pub fn read_body(&mut self) -> &mut BodyReader<'a, R> {
        assert!(self.state == ReaderState::Body);

        self.body_reader.as_mut().unwrap()
    }

    /// Finishes reading the message.
    ///
    /// [Self::begin_request] or [Self::begin_response] may be called next if
    /// the protocol allows it.
    ///
    ///
    ///
    /// Panics when called out of sequence.
    pub fn end_message(&mut self) -> Result<(), HTTPError> {
        tracing::debug!("end_message");
        assert!(self.state == ReaderState::Body);

        self.stream = Some(
            self.body_reader
                .take()
                .unwrap()
                .stream
                .into_inner()
                .into_inner(),
        );

        self.state = ReaderState::Header;

        Ok(())
    }

    /// Returns whether there has been a possible length mismatch.
    ///
    /// When Content-Length has specified and the reader is at EOF,
    /// this function will return true if the internal buffer is not empty.
    /// Otherwise, returns false.
    pub fn has_length_mismatch(&self) -> bool {
        // if let Some(content_length) = self.content_length {
        //     self.read_count == content_length && self.stream.get_ref().
        // }
        todo!()
    }
}

enum BodyTransportLayer<R: BufRead> {
    Chunked(ChunkedReader<R>),
    Length(ExpectedLengthReader<Take<R>>),
    Legacy(R),
}

impl<R: BufRead> BodyTransportLayer<R> {
    fn into_inner(self) -> R {
        match self {
            BodyTransportLayer::Chunked(stream) => stream.into_inner(),
            BodyTransportLayer::Length(stream) => stream.stream.into_inner(),
            BodyTransportLayer::Legacy(stream) => stream,
        }
    }
}

impl<R: BufRead> Read for BodyTransportLayer<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            BodyTransportLayer::Chunked(stream) => stream.read(buf),
            BodyTransportLayer::Length(stream) => stream.read(buf),
            BodyTransportLayer::Legacy(stream) => stream.read(buf),
        }
    }
}

struct ExpectedLengthReader<R: BufRead> {
    stream: R,
    current_length: u64,
    expected_length: u64,
}

impl<R: BufRead> Read for ExpectedLengthReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let amount = self.stream.read(buf)?;

        if amount == 0 && self.current_length != self.expected_length {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "content length mismatch",
            ))
        } else {
            self.current_length += amount as u64;

            Ok(amount)
        }
    }
}

/// Reader for a message body.
pub struct BodyReader<'a, R: BufRead> {
    stream: Decompressor<'a, BodyTransportLayer<R>>,
}

impl<'a, R: BufRead> Read for BodyReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}
