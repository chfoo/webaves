use std::{fmt::Display, io::Write};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    header::{HeaderFormatter, HeaderMap, HeaderParser},
    nomutil::NomParseError,
    string::StringLosslessExt,
};

use super::HTTPError;

const DEFAULT_VERSION: Version = (1, 1);

/// HTTP version in major decimal minor format.
pub type Version = (u16, u16);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestLine {
    pub method: String,
    pub target: String,
    pub version: Version,
}

impl RequestLine {
    pub fn new(method: String, target: String) -> Self {
        Self {
            method,
            target,
            version: DEFAULT_VERSION,
        }
    }

    pub fn parse_from(input: &[u8]) -> Result<Self, HTTPError> {
        match super::parse::parse_request_line(input) {
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

    pub fn format<W: Write>(&self, mut dest: W) -> Result<(), HTTPError> {
        self.validate()?;

        write!(
            &mut dest,
            "{} {} HTTP/{}.{}",
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusLine {
    pub version: Version,
    pub status_code: u16,
    pub reason_phrase: String,
}

impl StatusLine {
    pub fn new(status_code: u16) -> Self {
        Self {
            version: DEFAULT_VERSION,
            status_code,
            reason_phrase: String::default(),
        }
    }

    pub fn parse_from(input: &[u8]) -> Result<Self, HTTPError> {
        match super::parse::parse_status_line(input) {
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

    pub fn format<W: Write>(&self, mut dest: W) -> Result<(), HTTPError> {
        self.validate()?;

        write!(
            &mut dest,
            "HTTP/{}.{} {:03} {}",
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestHeader {
    pub request_line: RequestLine,
    pub fields: HeaderMap,
}

impl RequestHeader {
    pub fn new<M: Into<String>, T: Into<String>>(method: M, target: T) -> Self {
        Self {
            request_line: RequestLine::new(method.into(), target.into()),
            fields: HeaderMap::new(),
        }
    }

    pub fn parse_from(buf: &[u8]) -> Result<Self, HTTPError> {
        let (line, remain) = cut_start_line(buf);
        let request_line = RequestLine::parse_from(line)?;
        let field_buf = trim_trailing_newline(remain);
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

    pub fn format<W: Write>(&self, mut dest: W) -> Result<(), HTTPError> {
        self.request_line.format(&mut dest)?;
        dest.write_all(b"\r\n")?;

        let mut header_formatter = HeaderFormatter::new();
        header_formatter.use_raw(true);
        header_formatter
            .format_header(&self.fields, &mut dest)
            .map_err(|error| HTTPError::MalformedHeader {
                source: Some(Box::new(error)),
            })?;

        dest.write_all(b"\r\n")?;

        Ok(())
    }
}

impl Display for RequestHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.request_line.fmt(f)?;
        self.fields.fmt(f)?;
        f.write_str("\r\n")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseHeader {
    pub status_line: StatusLine,
    pub fields: HeaderMap,
}

impl ResponseHeader {
    pub fn new(status_code: u16) -> Self {
        Self {
            status_line: StatusLine::new(status_code),
            fields: HeaderMap::new(),
        }
    }

    pub fn parse_from(buf: &[u8]) -> Result<Self, HTTPError> {
        let (line, remain) = cut_start_line(buf);
        let status_line = StatusLine::parse_from(line)?;
        let field_buf = trim_trailing_newline(remain);
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

    pub fn format<W: Write>(&self, mut dest: W) -> Result<(), HTTPError> {
        self.status_line.format(&mut dest)?;
        dest.write_all(b"\r\n")?;

        let mut header_formatter = HeaderFormatter::new();
        header_formatter.use_raw(true);
        header_formatter
            .format_header(&self.fields, &mut dest)
            .map_err(|error| HTTPError::MalformedHeader {
                source: Some(Box::new(error)),
            })?;

        dest.write_all(b"\r\n")?;

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

/// Additional character classes.
pub(crate) trait HeaderByteExt {
    /// Returns whether the octet is valid as a "token" character.
    fn is_token(&self) -> bool;

    /// Returns whether the octet is classified as "obs-text".
    fn is_obs_text(&self) -> bool;
}

impl HeaderByteExt for u8 {
    fn is_token(&self) -> bool {
        self.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(self)
    }

    fn is_obs_text(&self) -> bool {
        *self >= 0x80
    }
}

fn cut_start_line(buf: &[u8]) -> (&[u8], &[u8]) {
    let index = buf
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(buf.len() - 1);
   buf.split_at(index + 1)
}

fn trim_trailing_newline(buf:&[u8]) -> &[u8]  {
    if buf.ends_with(b"\r\n") {
         &buf[0..buf.len() - 2]
    } else if buf.ends_with(b"\n") {
        &buf[0..buf.len() - 1]
    } else {
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let input = "GET /index.html HTTP/1.0\r\nk1: v1\r\n\r\n";
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

        assert_eq!(buf, b"POST /api/create HTTP/1.1\r\nk1: v1\r\n\r\n");
    }
}
