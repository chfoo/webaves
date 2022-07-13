//! Webaves web archive software suite.

#![warn(missing_docs)]
pub mod compress;
pub mod crypto;
pub mod dns;
pub mod download;
pub mod error;
pub mod fetch;
pub mod header;
pub mod http;
pub mod io;
pub mod net;
mod nomutil;
pub mod quest;
pub mod retry;
pub mod service;
pub mod stringesc;
pub mod stringutil;
pub mod tracker;
pub mod uuid;
pub mod warc;
