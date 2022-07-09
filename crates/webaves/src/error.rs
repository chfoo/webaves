//! Errors related to this crate.

use std::fmt::Display;

use thiserror::Error;

use crate::{http::HTTPError, nomutil::NomParseError};

/// General purpose error.
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP error.
    #[error(transparent)]
    HTTP(#[from] HTTPError),

    /// Parse error.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Miscellaneous error.
    #[error("{0}")]
    Misc(&'static str),

    /// Uncategorized error.
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Sync + Send>),
}

/// Error during parsing indicating malformed or invalid character sequences.
#[derive(Debug, Error)]
pub struct ParseError(pub(crate) NomParseError);

impl ParseError {
    /// Offset where the final error occurred in the input.
    pub fn offset(&self) -> u64 {
        self.0.offset()
    }

    /// A segment of the input near where the error occurred.
    pub fn input(&self) -> &[u8] {
        self.0.input()
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Formats an error chain to a string.
///
/// This function can be used to express error messages that pass outside
/// the Rust boundary.
pub fn format_to_string<E: std::error::Error>(error: E) -> String {
    let mut message = String::new();

    message.push_str(&error.to_string());

    let mut child_error = error.source();

    while let Some(error) = child_error {
        message.push_str(": ");
        message.push_str(&error.to_string());

        child_error = error.source();
    }

    message
}
