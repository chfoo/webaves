use std::fmt::Display;

use thiserror::Error;

use crate::nomutil::NomParseError;

use super::HeaderMap;

/// Error during parsing indicating malformed or invalid character sequences.
#[derive(Debug, Error)]
pub struct ParseError(NomParseError);

impl ParseError {
    /// Offset where the final error occurred in the input.
    pub fn offset(&self) -> u64 {
        self.0.offset()
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Decode header data into structures.
pub struct HeaderParser {
    // TODO: allow configuring to disable encoded-word
}

impl HeaderParser {
    /// Creates a `HeaderParser` with the default configuration.
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
        super::pc::parse_fields(input)
            .map_err(|error| ParseError(NomParseError::from_nom(input, &error)))
    }
}

impl Default for HeaderParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a field value formatted as a "parameter".
///
/// Example input: `name=value` or `name="value inside quoted-string"`.
pub fn parse_parameter(input: &[u8]) -> Result<(String, String), ParseError> {
    super::pc::parse_parameter(input)
        .map_err(|error| ParseError(NomParseError::from_nom(input, &error)))
}

/// Parse a field value formatted as a "quoted-string".
///
/// Example input: `"Hello world!"`.
pub fn parse_quoted_string(input: &[u8]) -> Result<String, ParseError> {
    super::pc::parse_quoted_string(input)
        .map_err(|error| ParseError(NomParseError::from_nom(input, &error)))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
