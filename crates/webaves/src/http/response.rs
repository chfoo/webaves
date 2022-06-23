use std::{fmt::Display, io::Write};

use serde::{Deserialize, Serialize};

use crate::{
    header::{HeaderFormatter, HeaderMap, HeaderParser},
    nomutil::NomParseError,
    string::StringLosslessExt,
};

use super::{util::HeaderByteExt, HTTPError, Version, DEFAULT_VERSION};

/// Represents a start line for a response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusLine {
    /// HTTP version.
    pub version: Version,
    /// Status code.
    pub status_code: u16,
    /// Reason phrase.
    pub reason_phrase: String,
}

impl StatusLine {
    /// Creates a new `StatusLine` with the given status code, default version,
    /// and empty reason phrase.
    pub fn new(status_code: u16) -> Self {
        Self {
            version: DEFAULT_VERSION,
            status_code,
            reason_phrase: String::default(),
        }
    }

    fn parse_from(input: &[u8]) -> Result<Self, HTTPError> {
        match super::pc::parse_status_line(input) {
            Ok(line) => Ok(Self {
                version: line.http_version,
                status_code: line.status_code,
                reason_phrase: String::from_utf8_lossless(line.reason_phrase),
            }),
            Err(error) => Err(HTTPError::InvalidStartLine {
                source: Some(Box::new(NomParseError::from_nom(input, &error))),
            }),
        }
    }

    fn format<W: Write>(&self, mut dest: W) -> Result<(), HTTPError> {
        self.validate()?;

        write!(
            &mut dest,
            "HTTP/{}.{} {:03} {}\r\n",
            self.version.0, self.version.1, self.status_code, self.reason_phrase
        )?;
        Ok(())
    }

    fn validate(&self) -> Result<(), HTTPError> {
        if !self
            .reason_phrase
            .as_bytes()
            .iter()
            .all(|c| b"\t ".contains(c) || c.is_ascii_graphic() || c.is_obs_text())
        {
            Err(HTTPError::InvalidStartLine { source: None })
        } else {
            Ok(())
        }
    }

    fn to_text_lossy(&self) -> String {
        format!(
            "HTTP/{}.{} {:03} {}",
            self.version.0,
            self.version.1,
            self.status_code,
            self.reason_phrase
                .replace(|c: char| c.is_ascii_control(), "\u{FFFD}")
        )
    }
}

impl Display for StatusLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_text_lossy())
    }
}

/// Represents the complete HTTP response header.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseHeader {
    /// Status line.
    pub status_line: StatusLine,
    /// Name-value fields.
    pub fields: HeaderMap,
}

impl ResponseHeader {
    /// Creates a `ResponseHeader` with the given status code, default version, and empty fields.
    pub fn new(status_code: u16) -> Self {
        Self {
            status_line: StatusLine::new(status_code),
            fields: HeaderMap::new(),
        }
    }

    /// Parses bytes into a new `ResponseHeader`.
    ///
    /// The given buffer must not contain the CRLF that separates the fields
    /// and message body.
    pub fn parse_from(buf: &[u8]) -> Result<Self, HTTPError> {
        let (line, field_buf) = super::util::cut_start_line(buf);
        let status_line = StatusLine::parse_from(line)?;
        let field_parser = HeaderParser::new();

        match field_parser.parse_header(field_buf) {
            Ok(fields) => Ok(Self {
                status_line,
                fields,
            }),
            Err(error) => Err(HTTPError::MalformedHeader {
                source: Some(Box::new(error)),
            }),
        }
    }

    /// Formats the header suitable network data exchange.
    ///
    /// The output is appended to `dest`. The output does not include the
    /// CRLF that separates the fields and message body.
    pub fn format<W: Write>(&self, mut dest: W) -> Result<(), HTTPError> {
        self.status_line.format(&mut dest)?;

        let mut header_formatter = HeaderFormatter::new();
        header_formatter.use_raw(true);
        header_formatter
            .format_header(&self.fields, &mut dest)
            .map_err(|error| HTTPError::MalformedHeader {
                source: Some(Box::new(error)),
            })?;

        Ok(())
    }
}

impl Display for ResponseHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.status_line.fmt(f)?;
        self.fields.fmt(f)?;
        f.write_str("\r\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response() {
        let input = b"HTTP/1.0 200 OK\r\nk1: v1\r\n";
        let response = ResponseHeader::parse_from(input).unwrap();

        assert_eq!(response.status_line.version, (1, 0));
        assert_eq!(response.status_line.status_code, 200);
        assert_eq!(response.status_line.reason_phrase, "OK");
        assert_eq!(response.fields.get_str("k1"), Some("v1"));
    }

    #[test]
    fn test_format_response() {
        let mut response = ResponseHeader::new(200);
        response.fields.insert("k1", "v1");
        response.status_line.reason_phrase = "OK".to_string();

        let mut buf = Vec::new();

        response.format(&mut buf).unwrap();

        assert_eq!(buf, b"HTTP/1.1 200 OK\r\nk1: v1\r\n");
    }
}
