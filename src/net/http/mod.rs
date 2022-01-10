pub mod client;
pub mod util;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("invalid request: {0}")]
    InvalidRequest(&'static str),

    #[error("incomplete: {0}")]
    Incomplete(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Header(#[from] http::Error),

    #[error(transparent)]
    HeaderParsing(#[from] httparse::Error),

    #[error(transparent)]
    Network(#[from] hyper::Error),
}

impl From<http::header::InvalidHeaderName> for HttpError {
    fn from(item: http::header::InvalidHeaderName) -> Self {
        item.into()
    }
}

impl From<http::header::InvalidHeaderValue> for HttpError {
    fn from(item: http::header::InvalidHeaderValue) -> Self {
        item.into()
    }
}

impl From<httparse::InvalidChunkSize> for HttpError {
    fn from(item: httparse::InvalidChunkSize) -> Self {
        item.into()
    }
}
