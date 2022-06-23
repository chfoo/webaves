use std::{fmt::Display, io::Write};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    header::{HeaderFormatter, HeaderMap, HeaderParser},
    nomutil::NomParseError,
    stringesc::StringLosslessExt,
    stringutil::CharClassExt,
};

use super::{HTTPError, Version, DEFAULT_VERSION};

/// Represents a start line for a request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestLine {
    /// Method name such as "GET" or "POST".
    pub method: String,
    /// The request-target.
    pub target: String,
    /// HTTP version.
    pub version: Version,
}

impl RequestLine {
    /// Creates a `RequestLine` with the given values and default version.
    pub fn new(method: String, target: String) -> Self {
        Self {
            method,
            target,
            version: DEFAULT_VERSION,
        }
    }

    fn parse_from(input: &[u8]) -> Result<Self, HTTPError> {
        match super::pc::parse_request_line(input) {
            Ok(line) => Ok(Self {
                method: String::from_utf8_lossless(line.method),
                target: String::from_utf8_lossless(line.request_target),
                version: line.http_version,
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
            "{} {} HTTP/{}.{}\r\n",
            self.method, self.target, self.version.0, self.version.1
        )?;
        Ok(())
    }

    fn validate(&self) -> Result<(), HTTPError> {
        if !self.method.as_bytes().iter().all(|c| c.is_token()) {
            Err(HTTPError::InvalidStartLine { source: None })
        } else {
            Ok(())
        }
    }

    fn to_text_lossy(&self) -> String {
        format!(
            "{} {} HTTP/{}.{}",
            self.method.replace(|c| !(c as u8).is_token(), "\u{FFFD}"),
            self.target,
            self.version.0,
            self.version.1
        )
    }
}

impl Display for RequestLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_text_lossy())
    }
}

/// Represents the types of RFC7230 request-target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestTarget {
    /// Direct request to server.
    Origin,
    /// Request to proxy.
    Absolute,
    /// CONNECT request.
    Authority,
    /// OPTIONS request.
    Asterisk,
}

/// Represents the complete HTTP request header.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestHeader {
    /// Request line.
    pub request_line: RequestLine,
    /// Name-value fields.
    pub fields: HeaderMap,
}

impl RequestHeader {
    /// Creates a `RequestHeader` with the given values, default version, and empty fields.
    pub fn new<M: Into<String>, T: Into<String>>(method: M, target: T) -> Self {
        Self {
            request_line: RequestLine::new(method.into(), target.into()),
            fields: HeaderMap::new(),
        }
    }

    /// Parses bytes into a new `RequestHeader`.
    ///
    /// The given buffer must not contain the CRLF that separates the fields
    /// and message body.
    pub fn parse_from(buf: &[u8]) -> Result<Self, HTTPError> {
        let (line, field_buf) = super::util::cut_start_line(buf);
        let request_line = RequestLine::parse_from(line)?;
        let field_parser = HeaderParser::new();

        match field_parser.parse_header(field_buf) {
            Ok(fields) => Ok(Self {
                request_line,
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
        self.request_line.format(&mut dest)?;

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

impl Display for RequestHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.request_line.fmt(f)?;
        f.write_str("\r\n")?;
        self.fields.fmt(f)
    }
}

/// Returns a request target for the given URL.
pub fn url_to_request_target(url: &Url, target: RequestTarget) -> String {
    match target {
        RequestTarget::Origin => {
            let mut path_and_query = url.path().to_string();

            if let Some(query) = url.query() {
                path_and_query.push('?');
                path_and_query.push_str(query);
            }

            path_and_query
        }
        RequestTarget::Absolute => url.to_string(),
        RequestTarget::Authority => format!(
            "{}:{}",
            url.host_str().unwrap_or_default(),
            url.port_or_known_default().unwrap_or_default()
        ),
        RequestTarget::Asterisk => "*".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let input = "GET /index.html HTTP/1.0\r\nk1: v1\r\n";
        let request = RequestHeader::parse_from(input.as_bytes()).unwrap();

        assert_eq!(request.request_line.method, "GET");
        assert_eq!(request.request_line.target, "/index.html");
        assert_eq!(request.request_line.version, (1, 0));
        assert_eq!(request.fields.get_str("k1"), Some("v1"));
    }

    #[test]
    fn test_format_request() {
        let mut request = RequestHeader::new("POST", "/api/create");
        request.fields.insert("k1", "v1");
        let mut buf = Vec::new();

        request.format(&mut buf).unwrap();

        assert_eq!(buf, b"POST /api/create HTTP/1.1\r\nk1: v1\r\n");
    }
}
