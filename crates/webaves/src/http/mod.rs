mod header;
mod parse;

pub use header::*;

use thiserror::Error;

/// Errors during parsing or formatting of WARC files.
#[derive(Error, Debug)]
pub enum HTTPError {
    /// Unexpected end of data.
    #[error("unexpected end of data")]
    UnexpectedEnd,

    /// Invalid or malformed start line (request line or status line).
    #[error("invalid start line")]
    InvalidStartLine {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Header couldn't be parsed or formatted.
    #[error("malformed header")]
    MalformedHeader {
        /// Source of the error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
