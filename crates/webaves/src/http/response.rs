use std::{fmt::Display, io::Write};

use serde::{Deserialize, Serialize};

use crate::{
    header::{HeaderFormatter, HeaderMap, HeaderParser},
    nomutil::NomParseError,
    string::StringLosslessExt,
};

use super::{util::HeaderByteExt, HTTPError, Version, DEFAULT_VERSION};

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
        let (line, remain) = super::util::cut_start_line(buf);
        let status_line = StatusLine::parse_from(line)?;
        let field_buf = super::util::trim_trailing_newline(remain);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "todo"]
    fn test_parse_response() {
        todo!()
    }

    #[test]
    #[ignore = "todo"]
    fn test_format_response() {
        todo!()
    }
}
