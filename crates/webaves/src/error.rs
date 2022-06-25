//! Errors related to this crate.

use std::fmt::Display;

use thiserror::Error;

use crate::nomutil::NomParseError;

/// General purpose error.
#[derive(Error, Debug)]
pub enum Error {
    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Error during parsing indicating malformed or invalid character sequences.
#[derive(Debug, Error)]
pub struct ParseError(pub(crate) NomParseError);

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
