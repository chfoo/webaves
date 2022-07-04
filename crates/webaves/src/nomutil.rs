use std::fmt::Display;

use nom::error::VerboseError;
use thiserror::Error;

/// Error during parsing.
#[derive(Debug, Error)]
pub struct NomParseError {
    offset: u64,
    input: Vec<u8>,
    message: String,
    // source: Option<Box<dyn std::error::Error>>,
}

impl NomParseError {
    pub fn from_nom(input: &[u8], error: &nom::Err<VerboseError<&[u8]>>) -> Self {
        let offset = get_parse_error_offset(input, error);
        let input_min = usize::try_from(((offset as i64) - 8).max(0)).unwrap_or_default();
        let input_max =
            usize::try_from(((offset as i64) + 8).min(input.len() as i64)).unwrap_or_default();
        let input = input[input_min..input_max].to_vec();

        Self {
            offset,
            input,
            message: get_parse_error_message(error),
        }
    }

    /// Offset where the final error occurred in the input.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// A segment of the input near where the error occurred.
    pub fn input(&self) -> &[u8] {
        self.input.as_ref()
    }
}

impl Display for NomParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "parse error at offset {}, near `{}`: {}",
            self.offset,
            String::from_utf8_lossy(&self.input).escape_debug(),
            self.message
        ))
    }
}

impl From<nom::Err<VerboseError<&[u8]>>> for NomParseError {
    fn from(error: nom::Err<VerboseError<&[u8]>>) -> Self {
        NomParseError {
            offset: 0,
            input: Vec::new(),
            message: crate::nomutil::get_parse_error_message(&error),
        }
    }
}

pub fn get_parse_error_offset(input: &[u8], error: &nom::Err<VerboseError<&[u8]>>) -> u64 {
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

pub fn get_parse_error_message(error: &nom::Err<VerboseError<&[u8]>>) -> String {
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
