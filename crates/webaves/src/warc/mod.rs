//! WARC file processing.
mod header;
mod reader;

pub use header::*;
pub use reader::*;

use thiserror::Error;

/// Errors during parsing or formatting of WARC files.
#[derive(Error, Debug)]
pub enum WARCError {
    /// Not a recognized WARC file.
    #[error("unknown format")]
    UnknownFormat,

    /// Header couldn't be parsed or formatted.
    #[error("malformed header")]
    MalformedHeader {
        /// Number of bytes read from the (uncompressed) input stream.
        offset: u64,
        /// Source of the error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// The length of the record body does not correspond with the value in the header.
    #[error("wrong block length")]
    WrongBlockLength {
        /// ID of the record
        record_id: String,
    },

    /// Field contained an invalid value.
    #[error("invalid field value")]
    InvalidFieldValue {
        /// Name of the field.
        name: String,
        /// ID of the record.
        record_id: String,
        /// Source of the error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// End of the record is malformed.
    #[error("malformed footer")]
    MalformedFooter {
        /// Number of bytes read from the (uncompressed) input stream.
        offset: u64,
    },

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}