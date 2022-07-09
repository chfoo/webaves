//! RPC helper utilities

use std::{marker::PhantomData, net::SocketAddr};

use serde::{Deserialize, Serialize};
use tarpc::{
    serde_transport::Transport,
    server::{BaseChannel, Channel, Serve},
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_serde::formats::Bincode;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::Instrument;

use crate::error::Error;

use super::Listen;

/// Helper to run a Tarpc service.
pub struct ServiceRunner<S, R, L, RW>
where
    S: Serve<R>,
    L: Listen<RW>,
    RW: AsyncRead + AsyncWrite,
{
    server: S,
    listener: L,

    _req: PhantomData<R>,
    _stream: PhantomData<RW>,
}

impl<S, R, L, RW> ServiceRunner<S, R, L, RW>
where
    S: Serve<R> + Send + Clone + 'static,
    S::Fut: Send,
    R: for<'de> Deserialize<'de> + Send + 'static,
    S::Resp: Serialize + Send + 'static,
    L: Listen<RW>,
    RW: AsyncRead + AsyncWrite + Send + 'static,
{
    /// Create a `ServerRunner` with the given service handler and listener.
    pub fn new(server: S, listener: L) -> Self {
        Self {
            server,
            listener,
            _req: PhantomData,
            _stream: PhantomData,
        }
    }

    /// Set the connection to listen for incoming connections.
    pub fn listen(&mut self) -> Result<Option<SocketAddr>, Error> {
        let local_address = self.listener.listen()?;

        match local_address {
            Some(local_address) => tracing::info!(?local_address, "server listening"),
            None => tracing::info!("server listening"),
        }

        Ok(local_address)
    }

    /// Start a loop to accept connections and process RPC requests.
    pub async fn accept_loop(&mut self) -> Result<(), Error> {
        loop {
            let (stream, remote_address) = self.listener.accept().await?;
            let server = self.server.clone();

            tokio::spawn(
                async move {
                    tracing::info!("connected");
                    let transport = create_transport(stream);
                    BaseChannel::with_defaults(transport).execute(server).await;
                    tracing::info!("disconnected");
                }
                .instrument(tracing::info_span!("client", ?remote_address)),
            );
        }
    }
}

/// Create a Tarpc Transport compatible with Webaves services.
pub fn create_transport<S, Item, SinkItem>(
    stream: S,
) -> Transport<S, Item, SinkItem, Bincode<Item, SinkItem>>
where
    S: AsyncWrite + AsyncRead,
    Item: for<'de> Deserialize<'de>,
    SinkItem: Serialize,
{
    let framed = Framed::new(stream, LengthDelimitedCodec::new());
    let codec = Bincode::default();

    tarpc::serde_transport::new(framed, codec)
}
