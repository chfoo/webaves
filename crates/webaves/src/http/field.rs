//! Header field values parsers.
use std::{collections::HashMap, str::FromStr};

use crate::{error::ParseError, header::HeaderMap, nomutil::NomParseError};

/// Parse a field value formatted as a "parameter".
///
/// Example input: `name=value` or `name="value inside quoted-string"`.
pub fn parse_parameter(input: &[u8]) -> Result<(String, String), ParseError> {
    super::pc::parse_parameter(input)
        .map_err(|error| ParseError::from(NomParseError::from_nom(input, &error)))
}

/// Parse a field value formatted as a "quoted-string".
///
/// Example input: `"Hello world!"`.
pub fn parse_quoted_string(input: &[u8]) -> Result<String, ParseError> {
    super::pc::parse_quoted_string(input)
        .map_err(|error| ParseError::from(NomParseError::from_nom(input, &error)))
}

/// Parse a field value formatted as a comma separated list.
///
/// Example input: `abc, "Hello world!"`.
pub fn parse_comma_list(input: &[u8]) -> Result<Vec<String>, ParseError> {
    super::pc::parse_comma_list(input)
        .map_err(|error| ParseError::from(NomParseError::from_nom(input, &error)))
}

/// Extension trait for header map for HTTP values.
pub trait HeaderMapExt {
    /// Returns values formatted as comma separated list or duplicate names.
    fn get_comma_list<N: Into<String>>(&self, name: N) -> Vec<String>;
}

impl HeaderMapExt for HeaderMap {
    fn get_comma_list<N: Into<String>>(&self, name: N) -> Vec<String> {
        let mut list = Vec::new();

        for field_value in self.get_all(name) {
            let values = match parse_comma_list(field_value.text.as_bytes()) {
                Ok(values) => values,
                Err(error) => {
                    tracing::trace!(?error, "get_comma_list");
                    Vec::new()
                }
            };

            list.extend_from_slice(&values);
        }

        list.iter_mut().for_each(|item| item.make_ascii_lowercase());

        list
    }
}

/// Represents the media-type value such as the Content-Type name.
pub struct MediaType {
    /// The "type".
    pub type_: String,
    /// The "subtype".
    pub subtype: String,
    /// The "parameters".
    pub parameters: HashMap<String, String>,
}

impl TryFrom<&[u8]> for MediaType {
    type Error = ParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        super::pc::parse_media_type(value)
            .map(|(type_, subtype, parameters)| {
                let parameters = HashMap::from_iter(parameters);
                Self {
                    type_,
                    subtype,
                    parameters,
                }
            })
            .map_err(|error| ParseError::from(NomParseError::from_nom(value, &error)))
    }
}

impl FromStr for MediaType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.as_bytes())
    }
}
