//! Errors related to this crate.

use thiserror::Error;

/// General purpose error.
#[derive(Error, Debug)]
pub enum Error {
    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
