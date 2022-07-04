use crate::nomutil::NomParseError;

use super::HeaderMap;

/// Error occured parsing header.
pub type ParseError = crate::error::ParseError;

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
            .map_err(|error| crate::error::ParseError(NomParseError::from_nom(input, &error)))
    }
}

impl Default for HeaderParser {
    fn default() -> Self {
        Self::new()
    }
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
