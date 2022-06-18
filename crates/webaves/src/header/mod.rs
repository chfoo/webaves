//! HTTP-style header name-value fields.
//!
//! Supports awareness of the following:
//!
//! - spaces before colon
//! - folded lines
//! - quoted-string
//! - encoded-word (RFC2047)
//!
//! Note that the data structures do not perform validation on their own and
//! are allowed to hold potentially malformed or invalid character sequences.
mod parse;

use std::{fmt::Display, io::Write, ops::Index};

use nom::error::VerboseError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::string::StringLosslessExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderMap {
    pairs: Vec<FieldPair>,
}

impl HeaderMap {
    pub fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator of all fields.
    pub fn iter(&self) -> HeaderMapIter<'_> {
        HeaderMapIter {
            map: self,
            index: 0,
        }
    }

    pub fn contains_key<N: Into<String>>(&self, name: N) -> bool {
        self.get(name).is_some()
    }

    /// Returns the first field value for the given name.
    pub fn get<N: Into<String>>(&self, name: N) -> Option<&FieldValue> {
        let mut name = name.into();
        name.make_ascii_lowercase();

        for pair in &self.pairs {
            if pair.name.normalized == name {
                return Some(&pair.value);
            }
        }

        None
    }

    /// Returns all the field values for the given name.
    pub fn get_all<N: Into<String>>(&self, name: N) -> FieldValuesIter<'_> {
        let mut name = name.into();
        name.make_ascii_lowercase();

        FieldValuesIter {
            name,
            map: self,
            index: 0,
        }
    }

    pub fn get_str<N: Into<String>>(&self, name: N) -> Option<&str> {
        match self.get(name) {
            Some(field) => Some(field.text.as_ref()),
            None => None,
        }
    }

    /// Add a field preserving any fields matching the given name.
    pub fn append<N, V>(&mut self, name: N, value: V)
    where
        N: Into<FieldName>,
        V: Into<FieldValue>,
    {
        self.pairs.push(FieldPair::new(name.into(), value.into()))
    }

    /// Remove any existing field with the given name and add the given field.
    pub fn insert<N, V>(&mut self, name: N, value: V)
    where
        N: Into<FieldName>,
        V: Into<FieldValue>,
    {
        let name = name.into();
        self.pairs
            .retain(|pair| pair.name.normalized != name.normalized);
        self.pairs.push(FieldPair::new(name, value.into()));
    }
}

impl Default for HeaderMap {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Into<String>> Index<N> for HeaderMap {
    type Output = FieldValue;

    fn index(&self, index: N) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl Display for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for pair in self.pairs.iter() {
            pair.fmt(f)?;
        }

        Ok(())
    }
}

/// Iterator for all fields.
pub struct HeaderMapIter<'a> {
    map: &'a HeaderMap,
    index: usize,
}

impl<'a> Iterator for HeaderMapIter<'a> {
    type Item = &'a FieldPair;

    fn next(&mut self) -> Option<Self::Item> {
        match self.map.pairs.get(self.index) {
            Some(item) => {
                self.index += 1;
                Some(item)
            }
            None => None,
        }
    }
}

/// Iterator of values for a name.
pub struct FieldValuesIter<'a> {
    name: String,
    map: &'a HeaderMap,
    index: usize,
}

impl<'a> Iterator for FieldValuesIter<'a> {
    type Item = &'a FieldValue;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.map.pairs.get(self.index) {
                Some(item) => {
                    self.index += 1;

                    if self.name == item.name.normalized {
                        return Some(&item.value);
                    }
                }
                None => return None,
            }
        }
    }
}

/// Represents a single name-value field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPair {
    /// The name or key.
    pub name: FieldName,
    /// The value.
    pub value: FieldValue,
}

impl FieldPair {
    pub fn new(name: FieldName, value: FieldValue) -> Self {
        Self { name, value }
    }
}

impl From<(FieldName, FieldValue)> for FieldPair {
    fn from(pair: (FieldName, FieldValue)) -> Self {
        FieldPair::new(pair.0, pair.1)
    }
}

impl Display for FieldPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name.to_text_lossy())?;
        f.write_str(": ")?;
        f.write_str(&self.value.to_text_lossy())?;
        f.write_str("\r\n")?;
        Ok(())
    }
}

/// Represents the name or key portion of a field.
///
/// The contents may be contain malformed or invalid sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldName {
    #[serde(skip)]
    normalized: String,

    /// Name decoded.
    pub text: String,

    /// Name in the original encoded format.
    pub raw: Option<Vec<u8>>,
}

impl FieldName {
    pub fn new(text: String, raw: Option<Vec<u8>>) -> Self {
        Self {
            normalized: text.to_ascii_lowercase(),
            text,
            raw,
        }
    }

    /// Returns a string with potential invalid characters replaced.
    ///
    /// This is intended for debugging purposes.
    pub fn to_text_lossy(&self) -> String {
        self.text.replace(|c| !(c as u8).is_token(), "\u{FFFD}")
    }
}

impl From<&str> for FieldName {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<String> for FieldName {
    fn from(value: String) -> Self {
        Self {
            normalized: value.to_ascii_lowercase(),
            text: value,
            raw: None,
        }
    }
}

impl From<&[u8]> for FieldName {
    fn from(value: &[u8]) -> Self {
        Self::from(value.to_vec())
    }
}

impl From<Vec<u8>> for FieldName {
    fn from(value: Vec<u8>) -> Self {
        let text = String::from_utf8_lossless(&value);

        Self {
            normalized: text.to_ascii_lowercase(),
            text: text.to_string(),
            raw: Some(value),
        }
    }
}

impl Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// Represents the value portion of a field.
///
/// The contents may be contain malformed or invalid sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    /// Value decoded.
    pub text: String,

    /// Value in the original encoded format.
    pub raw: Option<Vec<u8>>,
}

impl FieldValue {
    pub fn new(text: String, raw: Option<Vec<u8>>) -> Self {
        Self { text, raw }
    }

    /// Returns a string with potential invalid characters replaced.
    ///
    /// This is intended for debugging purposes.
    pub fn to_text_lossy(&self) -> String {
        self.text.replace(|c| c == '\r' || c == '\n', "\u{FFFD}")
    }
}

impl From<&str> for FieldValue {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        Self {
            text: value,
            raw: None,
        }
    }
}

impl From<&[u8]> for FieldValue {
    fn from(value: &[u8]) -> Self {
        Self::from(value.to_vec())
    }
}

impl From<Vec<u8>> for FieldValue {
    fn from(value: Vec<u8>) -> Self {
        Self {
            text: String::from_utf8_lossless(&value),
            raw: Some(value),
        }
    }
}

impl Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// Additional character classes.
pub trait HeaderByteExt {
    /// Returns whether the octet is valid as a "token" character.
    ///
    /// Any ASCII character except controls and separators.
    fn is_token(&self) -> bool;

    /// Returns whether the octet is a "separators" character.
    ///
    /// `( ) < > @ , ; : Backslash " / [ ] ? = { } Space Tab`
    fn is_separator(&self) -> bool;

    /// Returns whether the octet is valid as a "TEXT" character.
    ///
    /// Any octet except controls but including LWS.
    fn is_text(&self) -> bool;

    /// Returns whether the octet is valid as a linear whitespace "LWS" character.
    ///
    /// `CR LF Space Tab`
    fn is_lws(&self) -> bool;

    /// Returns the number of bytes in a UTF-8 sequence.
    ///
    /// - If 1, then the octet encodes itself.
    /// - If 2, then the octet encodes itself and 1 following octet.
    /// - If 3, then the octet encodes itself and 2 following octets.
    /// - If 4, then the octet encodes itself and 3 following octets.
    /// - Otherwise, 0, invalid encoding.
    fn sequence_length(&self) -> u32;
}

impl HeaderByteExt for u8 {
    fn is_token(&self) -> bool {
        self.is_ascii() && !self.is_ascii_control() && !self.is_separator()
    }

    fn is_separator(&self) -> bool {
        b"()<>@,;:\\\"/[]?={} \t".contains(self)
    }

    fn is_text(&self) -> bool {
        !self.is_ascii_control() || b"\r\n \t".contains(self)
    }

    fn is_lws(&self) -> bool {
        b"\r\n \t".contains(self)
    }

    fn sequence_length(&self) -> u32 {
        match self.leading_ones() {
            0 => 1,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            _ => 0,
        }
    }
}

#[derive(Error, Debug)]
pub enum FormatError {
    #[error(transparent)]
    Data(#[from] FormatDataError),

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
    pub fn use_raw(&mut self, value: bool) -> &mut Self {
        self.use_raw = value;
        self
    }

    /// Sets whether invalid character sequences are checked.
    ///
    /// If true, the formatter will not return an error when encountering
    /// invalid character sequences. Enabling this feature will introduce security
    /// vulnerabilities.
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

/// Error during parsing indicating malformed or invalid character sequences.
#[derive(Debug, Error)]
pub struct ParseError {
    offset: u64,
    message: String,
    // source: Option<Box<dyn std::error::Error>>,
}

impl ParseError {
    /// Offset where the final error occurred in the input.
    pub fn offset(&self) -> u64 {
        self.offset
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "parse error at offset {}: {}",
            self.offset, self.message
        ))
    }
}

/// Decode header data into structures.
pub struct HeaderParser {
    // TODO: allow configuring to disable encoded-word
}

impl HeaderParser {
    pub fn new() -> Self {
        Self {}
    }

    /// Decode the given header data.
    ///
    /// Special decoding steps are performed:
    ///
    /// - Whitespace surrounding names and values are removed.
    /// - Folded lines are automatically unfolded.
    /// - Characters in quoted-string encoding is respected.
    /// - Characters in encoded-word (RFC2047) encoding are decoded
    ///   if possible, otherwise unchanged.
    pub fn parse_header(&self, input: &[u8]) -> Result<HeaderMap, ParseError> {
        self::parse::parse(input).map_err(|error| ParseError {
            offset: get_parse_error_offset(input, &error),
            message: get_parse_error_message(&error),
            // source: Some(Box::new(error)),
        })
    }
}

impl Default for HeaderParser {
    fn default() -> Self {
        Self::new()
    }
}

fn get_parse_error_offset(input: &[u8], error: &nom::Err<VerboseError<&[u8]>>) -> u64 {
    match error {
        nom::Err::Incomplete(_) => input.len() as u64,
        nom::Err::Error(error) => get_error_offset_from_list(input, &error.errors),
        nom::Err::Failure(error) => get_error_offset_from_list(input, &error.errors),
    }
}

fn get_error_offset_from_list(
    input: &[u8],
    errors: &[(&[u8], nom::error::VerboseErrorKind)],
) -> u64 {
    (input.len() - errors.first().map(|e| e.0.len()).unwrap_or_default()) as u64
}

fn get_parse_error_message(error: &nom::Err<VerboseError<&[u8]>>) -> String {
    match error {
        nom::Err::Incomplete(_error) => "incomplete".to_string(),
        nom::Err::Error(error) => get_parse_error_kind_message(&error.errors),
        nom::Err::Failure(error) => get_parse_error_kind_message(&error.errors),
    }
}

fn get_parse_error_kind_message(errors: &[(&[u8], nom::error::VerboseErrorKind)]) -> String {
    errors
        .first()
        .map(|e| match e.1 {
            nom::error::VerboseErrorKind::Context(error) => error.to_string(),
            nom::error::VerboseErrorKind::Char(c) => format!("expected char {}", c),
            nom::error::VerboseErrorKind::Nom(error) => error.description().to_string(),
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_get_convenience() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("k2", "v2");

        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
        assert!(map.contains_key("k1"));
        assert!(map.contains_key("k2"));
        assert!(!map.contains_key("k3"));
        assert_eq!(map.get_str("k1"), Some("v1"));
        assert_eq!(map.get_str("k2"), Some("v2"));
        assert_eq!(map.get_str("k3"), None);
    }

    #[test]
    fn test_header_map_uniques() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("k2", "v2");

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("k1").map(|v| v.text.as_ref()), Some("v1"));
        assert_eq!(map.get("k2").map(|v| v.text.as_ref()), Some("v2"));

        map.insert("k2", "hello world");

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("k1").map(|v| v.text.as_ref()), Some("v1"));
        assert_eq!(map.get("k2").map(|v| v.text.as_ref()), Some("hello world"));
    }

    #[test]
    fn test_header_map_duplicates() {
        let mut map = HeaderMap::new();

        map.append("k1", "v1");
        map.append("k2", "v2");
        map.append("k1", "v3");
        map.append("k2", "v4");

        assert_eq!(map.len(), 4);
        assert_eq!(map.get("k1").map(|v| v.text.as_ref()), Some("v1"));
        assert_eq!(map.get("k2").map(|v| v.text.as_ref()), Some("v2"));
        assert_eq!(
            map.get_all("k1")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["v1", "v3"]
        );
        assert_eq!(
            map.get_all("k2")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["v2", "v4"]
        );

        map.insert("k1", "hello world");

        assert_eq!(map.len(), 3);
        assert_eq!(
            map.get_all("k1")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["hello world"]
        );
        assert_eq!(
            map.get_all("k2")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["v2", "v4"]
        );
    }

    #[test]
    fn test_header_iter() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("k2", "v2");

        assert_eq!(map.len(), 2);

        let pairs = map.iter().cloned().collect::<Vec<FieldPair>>();

        assert_eq!(pairs[0].name.text, "k1");
        assert_eq!(pairs[0].value.text, "v1");
        assert_eq!(pairs[1].name.text, "k2");
        assert_eq!(pairs[1].value.text, "v2");
    }

    #[test]
    fn test_header_map_case_sensitivity() {
        let mut map = HeaderMap::new();

        map.insert("Hello-World", "v1");
        map.insert("hello-world", "v2");

        assert_eq!(map.len(), 1);
        assert_eq!(map.get("HELLO-WORLD").map(|v| v.text.as_ref()), Some("v2"));

        map.append("HELLO-world", "v3");

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get_all("hello-WORLD")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["v2", "v3"]
        );
    }

    #[test]
    fn test_display() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("k2", "v2");

        let result = map.to_string();

        assert_eq!(result, "k1: v1\r\nk2: v2\r\n");
    }

    #[test]
    fn test_header_byte_ext() {
        assert!(b'a'.is_token());
        assert!(!b'\n'.is_token());
        assert!(!b':'.is_token());

        assert!(b':'.is_separator());
        assert!(!b'a'.is_separator());

        assert!(b'a'.is_text());
        assert!(b'\t'.is_text());
        assert!(b'\n'.is_text());
        assert!(!b'\x00'.is_text());

        assert!(b'\t'.is_lws());
        assert!(!b'a'.is_lws());

        assert_eq!(b'a'.sequence_length(), 1);
        assert_eq!(b'\x80'.sequence_length(), 1);
        assert_eq!(b'\xC4'.sequence_length(), 2);
        assert_eq!(b'\xE3'.sequence_length(), 3);
        assert_eq!(b'\xF0'.sequence_length(), 4);
        assert_eq!(b'\xFF'.sequence_length(), 0);
    }

    #[test]
    fn test_parse_ok() {
        let data = b"k1: v1\r\n";

        assert!(HeaderParser::new().parse_header(data).is_ok());
    }

    #[test]
    fn test_parse_err() {
        let data = b"k1: v1\r\nk2";
        let error = HeaderParser::new().parse_header(data).unwrap_err();

        assert_eq!(error.offset(), 8);
    }

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
