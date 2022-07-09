//! Network and connections.
mod local;
pub mod rpc;

pub use local::*;

use std::net::SocketAddr;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Error;

/// Abstraction for a client connection.
///
/// Implementations should automatically handle temporary OS errors such as
/// errors during a brief moment the server is busy accepting a client.
///
/// Note: This is a `async_trait`.
#[async_trait::async_trait]
pub trait Connect<S>
where
    S: AsyncRead + AsyncWrite,
{
    /// Connect to the service and return a stream.
    ///
    /// Equivalent to:
    ///
    /// ```ignore
    /// async fn connect(&self) -> Result<S, Error>;
    /// ```
    async fn connect(&self) -> Result<S, Error>;
}

/// Abstraction for a server connection.
///
/// Implementations should automatically handle temporary OS errors such as
/// an error accepting a connection because it is already closed.
///
/// Note: This is a `async_trait`.
#[async_trait::async_trait]
pub trait Listen<S>
where
    S: AsyncRead + AsyncWrite,
{
    /// Begin listening for client connections to the host.
    fn listen(&mut self) -> Result<Option<SocketAddr>, Error>;

    /// Wait for a client connection.
    ///
    /// Equivalent to:
    ///
    /// ```ignore
    /// async fn accept(&mut self) -> Result<(S, Option<SocketAddr>), Error>;
    /// ```
    async fn accept(&mut self) -> Result<(S, Option<SocketAddr>), Error>;
}
