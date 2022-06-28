//! HTTP parsing, connection handling, client and server.
pub mod chunked;
pub mod field;
mod pc;
mod request;
mod response;
mod util;

pub use request::*;
pub use response::*;

use thiserror::Error;

use crate::compress::CompressionFormat;

/// Default HTTP version.
pub const DEFAULT_VERSION: Version = (1, 1);

/// HTTP version in major decimal minor format.
pub type Version = (u16, u16);

/// Specifies what compression method to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionOption {
    /// Don't use any compression.
    None,
    /// Detect compression from headers.
    Auto,
    /// Use specified compression format.
    Manual(CompressionFormat),
}

impl Default for CompressionOption {
    fn default() -> Self {
        Self::Auto
    }
}

/// Specifies the use of chunked transfer coding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkedEncodingOption {
    /// Do not use.
    Off,
    /// Always use.
    On,
    /// Detect from headers.
    Auto,
}

impl Default for ChunkedEncodingOption {
    fn default() -> Self {
        Self::Auto
    }
}

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

    /// Invalid or malformed sequence in content encoding or transfer coding.
    #[error("invalid encoding")]
    InvalidEncoding {
        /// Source of the error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Feature or condition is not supported by this crate.
    #[error("not supported")]
    NotSupported {
        /// Source of the error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
