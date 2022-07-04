//! WARC file processing.
pub mod extract;
mod header;
mod reader;
mod writer;

pub use header::*;
pub use reader::*;
pub use writer::*;

use thiserror::Error;

/// Errors during parsing or formatting of WARC files.
#[derive(Error, Debug)]
pub enum WARCError {
    /// Not a recognized WARC file.
    #[error("unknown format")]
    UnknownFormat,

    /// Header couldn't be parsed or formatted.
    #[error("malformed header (at offset {offset})")]
    MalformedHeader {
        /// Number of bytes read from the (uncompressed) input stream.
        offset: u64,
        /// Source of the error.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// The length of the record body does not correspond with the value in the header.
    #[error("wrong block length (at record ID {record_id}")]
    WrongBlockLength {
        /// ID of the record
        record_id: String,
    },

    /// Field contained an invalid value.
    #[error("invalid field value (with name {name}, at record ID {record_id})")]
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
    #[error("malformed footer (at offset {offset})")]
    MalformedFooter {
        /// Number of bytes read from the (uncompressed) input stream.
        offset: u64,
    },

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
