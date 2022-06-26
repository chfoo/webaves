//! HTTP parsing, connection handling, client and server.
mod header;
mod pc;
mod request;
mod response;
pub mod chunked;
mod util;

pub use header::*;
pub use request::*;
pub use response::*;

use thiserror::Error;

/// Errors during HTTP parsing, formatting, or processing protocol state.
#[derive(Error, Debug)]
pub enum HTTPError {
    /// Unexpected end of data.
    #[error("unexpected end of data")]
    UnexpectedEnd,

    /// Invalid or malformed start line (request line or status line).
    #[error("invalid start line")]
    InvalidStartLine {
        /// Source of the error.
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
