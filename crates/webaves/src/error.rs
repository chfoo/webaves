//! Errors related to this crate.

use std::fmt::Display;

use thiserror::Error;

use crate::{http::HTTPError, nomutil::NomParseError};

/// General purpose error.
#[derive(Error, Debug)]
pub enum Error {
    /// Protocol error.
    #[error(transparent)]
    Protocol(Box<dyn std::error::Error + Sync + Send>),

    /// Parse error.
    #[error(transparent)]
    Parse(ParseError),

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Uncategorized error.
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Sync + Send>),
}

impl From<HTTPError> for Error {
    fn from(error: HTTPError) -> Self {
        Self::Protocol(Box::new(error))
    }
}

/// Error during parsing indicating malformed or invalid character sequences.
#[derive(Debug, Error)]
pub struct ParseError(ParseErrorImpl);

impl ParseError {
    /// Offset where the final error occurred in the input.
    pub fn offset(&self) -> u64 {
        match &self.0 {
            ParseErrorImpl::Nom(error) => error.offset(),
            ParseErrorImpl::Other(_) => 0,
        }
    }

    /// A segment of the input near where the error occurred.
    pub fn input(&self) -> &[u8] {
        match &self.0 {
            ParseErrorImpl::Nom(error) => error.input(),
            ParseErrorImpl::Other(_) => b"",
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            ParseErrorImpl::Nom(error) => error.fmt(f),
            ParseErrorImpl::Other(message) => f.write_str(message),
        }
    }
}

impl From<NomParseError> for ParseError {
    fn from(error: NomParseError) -> Self {
        Self(ParseErrorImpl::Nom(error))
    }
}

impl From<&str> for ParseError {
    fn from(error: &str) -> Self {
        Self(ParseErrorImpl::Other(error.to_string()))
    }
}

impl From<String> for ParseError {
    fn from(error: String) -> Self {
        Self(ParseErrorImpl::Other(error))
    }
}

#[derive(Debug)]
enum ParseErrorImpl {
    Nom(NomParseError),
    Other(String),
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
