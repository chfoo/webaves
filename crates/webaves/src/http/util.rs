use crate::{error::ParseError, nomutil::NomParseError};

/// Splits a header into the first line and remainder.
pub fn cut_start_line(buf: &[u8]) -> (&[u8], &[u8]) {
    let index = buf
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(buf.len() - 1);
    buf.split_at(index + 1)
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
