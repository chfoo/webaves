//! Webaves web archive software suite.

#![warn(missing_docs)]
pub mod compress;
pub mod crypto;
pub mod dns;
pub mod download;
pub mod error;
pub mod header;
pub mod http;
pub mod io;
mod nomutil;
pub mod service;
pub mod stringesc;
pub mod stringutil;
pub mod uuid;
pub mod warc;
