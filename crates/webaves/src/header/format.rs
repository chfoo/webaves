use std::{fmt::Display, io::Write};

use thiserror::Error;

use crate::string::StringLosslessExt;

use super::{FieldName, FieldPair, FieldValue, HeaderByteExt, HeaderMap};

/// Represents an error that may occur during formatting of a [HeaderMap].
#[derive(Error, Debug)]
pub enum FormatError {
    /// Input data error.
    #[error(transparent)]
    Data(#[from] FormatDataError),

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Error during formatting indicating invalid character sequences.
#[derive(Debug)]
pub struct FormatDataError {
    line: u64,
    name: Option<FieldName>,
    value: Option<FieldValue>,
}

impl FormatDataError {
    /// Position of the name value field.
    pub fn line(&self) -> u64 {
        self.line
    }

    /// The invalid name field.
    pub fn name(&self) -> Option<&FieldName> {
        self.name.as_ref()
    }

    /// The invalid value field.
    pub fn value(&self) -> Option<&FieldValue> {
        self.value.as_ref()
    }
}

impl std::error::Error for FormatDataError {}

impl Display for FormatDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid name-value field")?;
        f.write_fmt(format_args!(" line {}", self.line))?;

        if let Some(name) = &self.name {
            f.write_fmt(format_args!(" name '{}'", name.text))?;
        }

        if let Some(value) = &self.value {
            f.write_fmt(format_args!(" value '{}'", value.text))?;
        }

        Ok(())
    }
}

impl From<std::fmt::Error> for FormatDataError {
    fn from(_: std::fmt::Error) -> Self {
        FormatDataError {
            line: 0,
            name: None,
            value: None,
        }
    }
}

/// Encodes headers suitable for network data exchange.
pub struct HeaderFormatter {
    lossless_scheme: bool,
    use_raw: bool,
    disable_validation: bool,
}

impl HeaderFormatter {
    /// Creates a new `HeaderFormatter` with the default configuration.
    pub fn new() -> Self {
        Self {
            lossless_scheme: false,
            use_raw: false,
            disable_validation: false,
        }
    }

    /// Sets whether text is encoded using [StringLosslessExt::to_utf8_lossless].
    ///
    /// This should only be true if header fields are constructed from sources
    /// that use [StringLosslessExt::from_utf8_lossless]. These include
    /// [HeaderParser] or manually from `From<&[u8]>`/`From<Vec<u8>>`.
    ///
    /// Default is false.
    pub fn lossless_scheme(&mut self, value: bool) -> &mut Self {
        self.lossless_scheme = value;
        self
    }

    /// Sets whether raw values are used when available.
    ///
    /// When true, if a name or value field's `raw` member is not None, it will
    /// be used instead of the `text` member.
    ///
    /// Sources such as [HeaderParser] include decoded text values that may
    /// significantly differ than the raw value.
    ///
    /// Default is false.
    pub fn use_raw(&mut self, value: bool) -> &mut Self {
        self.use_raw = value;
        self
    }

    /// Sets whether invalid character sequences are checked.
    ///
    /// If true, the formatter will not return an error when encountering
    /// invalid character sequences. Enabling this feature will introduce security
    /// vulnerabilities.
    ///
    /// Default is false.
    pub fn disable_validation(&mut self, value: bool) -> &mut Self {
        self.disable_validation = value;
        self
    }

    /// Format the name-value fields to HTTP-style format.
    ///
    /// Returns the number of bytes written.
    pub fn format_header<W: Write>(
        &self,
        header: &HeaderMap,
        mut dest: W,
    ) -> Result<usize, FormatError> {
        let mut num_bytes = 0;
        let mut temp = Vec::new();

        for (line, pair) in header.iter().enumerate() {
            let name_bytes = self.get_name_bytes(pair, &mut temp);

            self.validate_name(pair, name_bytes, line as u64)?;

            dest.write_all(name_bytes)?;

            if self.use_raw && pair.value.raw.is_some() {
                dest.write_all(b":")?;
                num_bytes += name_bytes.len() + 1;
            } else {
                dest.write_all(b": ")?;
                num_bytes += name_bytes.len() + 2;
            }

            let value_bytes = self.get_value_bytes(pair, &mut temp);

            self.validate_value(pair, value_bytes, line as u64)?;

            dest.write_all(value_bytes)?;
            dest.write_all(b"\r\n")?;
            num_bytes += value_bytes.len() + 2;
        }

        Ok(num_bytes)
    }

    fn get_name_bytes<'a>(&self, pair: &'a FieldPair, temp: &'a mut Vec<u8>) -> &'a [u8] {
        match pair.name.raw.as_ref() {
            Some(raw) if self.use_raw => raw.as_slice(),
            _ => {
                if self.lossless_scheme {
                    *temp = pair.name.text.to_utf8_lossless();
                    temp
                } else {
                    pair.name.text.as_bytes()
                }
            }
        }
    }

    fn get_value_bytes<'a>(&self, pair: &'a FieldPair, temp: &'a mut Vec<u8>) -> &'a [u8] {
        match pair.value.raw.as_ref() {
            Some(raw) if self.use_raw => raw.as_slice(),
            _ => {
                if self.lossless_scheme {
                    *temp = pair.value.text.to_utf8_lossless();
                    temp
                } else {
                    pair.value.text.as_bytes()
                }
            }
        }
    }

    fn validate_name(
        &self,
        pair: &FieldPair,
        name_bytes: &[u8],
        line: u64,
    ) -> Result<(), FormatError> {
        if !self.disable_validation && !name_bytes.iter().all(|c| c.is_token()) {
            return Err(FormatDataError {
                line,
                name: Some(pair.name.clone()),
                value: None,
            }
            .into());
        }

        Ok(())
    }

    fn validate_value(
        &self,
        pair: &FieldPair,
        value_bytes: &[u8],
        line: u64,
    ) -> Result<(), FormatError> {
        if !self.disable_validation && !value_bytes.iter().all(|&c| c != b'\r' && c != b'\n') {
            return Err(FormatDataError {
                line: line as u64,
                name: None,
                value: Some(pair.value.clone()),
            }
            .into());
        }

        Ok(())
    }
}

impl Default for HeaderFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");

        let mut buf = Vec::new();
        let formatter = HeaderFormatter::new();

        formatter.format_header(&map, &mut buf).unwrap();

        assert_eq!(buf, b"k1: v1\r\n");
    }

    #[test]
    fn test_format_lossless() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1\u{FFFD}\u{1055FF}");

        let mut buf = Vec::new();
        let mut formatter = HeaderFormatter::new();
        formatter.lossless_scheme(true);

        formatter.format_header(&map, &mut buf).unwrap();

        assert_eq!(buf, b"k1: v1\xff\r\n");
    }

    #[test]
    fn test_format_raw() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");

        map.insert(
            FieldName::new("k2".to_string(), Some(b"K2".to_vec())),
            FieldValue::new("v2".to_string(), Some(b"\tv2".to_vec())),
        );

        let mut buf = Vec::new();
        let mut formatter = HeaderFormatter::new();
        formatter.use_raw(true);

        formatter.format_header(&map, &mut buf).unwrap();
        assert_eq!(buf, b"k1: v1\r\nK2:\tv2\r\n");
    }

    #[test]
    fn test_format_invalid_key() {
        let mut map = HeaderMap::new();

        map.insert("k1:", "v1");

        let mut buf = Vec::new();
        let mut formatter = HeaderFormatter::new();

        let result = formatter.format_header(&map, &mut buf);
        assert!(result.is_err());

        buf.clear();
        formatter.disable_validation(true);
        formatter.format_header(&map, &mut buf).unwrap();
        assert_eq!(buf, b"k1:: v1\r\n");
    }

    #[test]
    fn test_format_invalid_value() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1\n");

        let mut buf = Vec::new();
        let mut formatter = HeaderFormatter::new();

        let result = formatter.format_header(&map, &mut buf);
        assert!(result.is_err());

        buf.clear();
        formatter.disable_validation(true);
        formatter.format_header(&map, &mut buf).unwrap();
        assert_eq!(buf, b"k1: v1\n\r\n");
    }
}
