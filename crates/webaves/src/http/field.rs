//! Header field values parsers.
use crate::{error::ParseError, nomutil::NomParseError};

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

/// Parse a field value formatted as a comma separated list.
///
/// Example input: `abc, "Hello world!"`.
pub fn parse_comma_list(input: &[u8]) -> Result<Vec<String>, ParseError> {
    super::pc::parse_comma_list(input)
        .map_err(|error| ParseError(NomParseError::from_nom(input, &error)))
}
