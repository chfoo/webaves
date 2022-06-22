use std::fmt::Display;

use nom::error::VerboseError;
use thiserror::Error;

/// Error during parsing.
#[derive(Debug, Error)]
pub struct NomParseError {
    offset: u64,
    message: String,
    // source: Option<Box<dyn std::error::Error>>,
}

impl NomParseError {
    pub fn from_nom(input: &[u8], error: &nom::Err<VerboseError<&[u8]>>) -> Self {
        Self {
            offset: get_parse_error_offset(input, error),
            message: get_parse_error_message(error),
        }
    }

    /// Offset where the final error occurred in the input.
    pub fn offset(&self) -> u64 {
        self.offset
    }
}

impl Display for NomParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "parse error at offset {}: {}",
            self.offset, self.message
        ))
    }
}

impl From<nom::Err<VerboseError<&[u8]>>> for NomParseError {
    fn from(error: nom::Err<VerboseError<&[u8]>>) -> Self {
        NomParseError {
            offset: 0,
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
