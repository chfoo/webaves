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
    HTTPError, RequestHeader, ResponseHeader, ZeroNineOption,
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
    zero_nine: ZeroNineOption,
    header_limit: u64,
    state: ReaderState,
    buffer: Vec<u8>,
    content_length: Option<u64>,
    server_is_modern: bool,
}

impl<'a, R: BufRead + PeekRead> MessageReader<'a, R> {
    /// Creates a new `MessageReader` with the given stream.
    pub fn new(stream: R) -> Self {
        Self {
            stream: Some(stream),
            body_reader: None,
            chunked_encoding: Default::default(),
            compression: Default::default(),
            zero_nine: Default::default(),
            header_limit: 65536,
            state: ReaderState::Header,
            buffer: Vec::new(),
            content_length: None,
            server_is_modern: false,
        }
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &R {
        match self.stream.as_ref() {
            Some(stream) => stream,
            None => self.body_reader.as_ref().unwrap().get_ref(),
        }
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut R {
        match self.stream.as_mut() {
            Some(stream) => stream,
            None => self.body_reader.as_mut().unwrap().get_mut(),
        }
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> R {
        match self.stream {
            Some(stream) => stream,
            None => self.body_reader.unwrap().into_inner(),
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

    /// Returns the HTTP/0.9 option for 0.9 responses.
    pub fn zero_nine(&self) -> ZeroNineOption {
        self.zero_nine
    }

    /// Sets the HTTP/0.9 option for 0.9 responses.
    pub fn set_zero_nine(&mut self, zero_nine: ZeroNineOption) {
        self.zero_nine = zero_nine;
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

        let header = if self.check_use_modern_headers()? {
            self.read_header()?;
            ResponseHeader::parse_from(crate::stringutil::trim_trailing_crlf(&self.buffer))?
        } else {
            tracing::debug!("using HTTP/0.9");
            ResponseHeader::new_09()
        };

        if !self.server_is_modern && header.status_line.version.0 >= 1 {
            tracing::trace!("mark server as modern");
            self.server_is_modern = true;
        }

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

    fn check_is_http_header(&mut self) -> Result<bool, HTTPError> {
        tracing::trace!("check_is_http_header");

        let stream = self.stream.as_mut().unwrap();
        let mut buffer = [0u8; 5];

        match stream.peek_exact(5) {
            Ok(data) => {
                buffer.copy_from_slice(data);
                buffer.make_ascii_lowercase();

                tracing::trace!(?buffer, "check_is_http_header");
                Ok(buffer.starts_with(b"http/"))
            }
            Err(error) => {
                tracing::trace!(?error, "check_is_http_header");
                Ok(false)
            }
        }
    }

    fn check_use_modern_headers(&mut self) -> Result<bool, HTTPError> {
        let is_http_header = self.check_is_http_header()?;

        tracing::trace!("check_http_response_magic_bytes");

        Ok(self.zero_nine == ZeroNineOption::Never || is_http_header || self.server_is_modern)
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
    /// Panics when called out of sequence.
    ///
    /// If there is remaining data to be read, this reader does not determine
    /// whether it belongs to the current message. This extra data may be the
    /// result of an incorrect Content-Length.
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
}

enum BodyTransportLayer<R: BufRead> {
    Chunked(ChunkedReader<R>),
    Length(ExpectedLengthReader<Take<R>>),
    Legacy(R),
}

impl<R: BufRead> BodyTransportLayer<R> {
    fn get_ref(&self) -> &R {
        match self {
            BodyTransportLayer::Chunked(stream) => stream.get_ref(),
            BodyTransportLayer::Length(stream) => stream.stream.get_ref(),
            BodyTransportLayer::Legacy(stream) => stream,
        }
    }

    fn get_mut(&mut self) -> &mut R {
        match self {
            BodyTransportLayer::Chunked(stream) => stream.get_mut(),
            BodyTransportLayer::Length(stream) => stream.stream.get_mut(),
            BodyTransportLayer::Legacy(stream) => stream,
        }
    }

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

impl<'a, R: BufRead> BodyReader<'a, R> {
    fn get_ref(&self) -> &R {
        self.stream.get_ref().get_ref()
    }

    fn get_mut(&mut self) -> &mut R {
        self.stream.get_mut().get_mut()
    }

    fn into_inner(self) -> R {
        self.stream.into_inner().into_inner()
    }
}

impl<'a, R: BufRead> Read for BodyReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}
