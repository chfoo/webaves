//! DNS resolution service.

use std::sync::{Arc};

use serde::{Deserialize, Serialize};
use tarpc::context::Context;
use tokio::sync::Mutex;

use crate::dns::{AddressResponse, Resolver, ResolverError};

/// Name of the service.
pub const SERVICE_NAME: &str = "dns-resolver";

/// DNS resolver RPC interface.
#[tarpc::service]
pub trait ResolverRPC {
    /// Lookup IP addresses for the given hostname.
    async fn lookup_address(hostname: String) -> Result<AddressResponse, ResolverRPCError>;
}

/// Errors returned from the RPC service.
#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
pub enum ResolverRPCError {
    /// Non-existent domain.
    #[error("non-existent domain")]
    NoName,

    /// No records for given record type.
    #[error("no records for given record type")]
    NoRecord,

    /// Any other error.
    #[error("other: {0}")]
    Other(String),
}

impl From<ResolverError> for ResolverRPCError {
    fn from(source: ResolverError) -> Self {
        match source {
            ResolverError::NoName => Self::NoName,
            ResolverError::NoRecord => Self::NoRecord,
            ResolverError::Other(error) => Self::Other(error.to_string()),
            ResolverError::Io(error) => Self::Other(crate::error::format_to_string(error)),
            ResolverError::OtherInternal(error) => {
                Self::Other(crate::error::format_to_string(error))
            }
        }
    }
}

/// DNS resolver RPC server.
#[derive(Clone)]
pub struct ResolverRPCServer {
    resolver: Arc<Mutex<Resolver>>,
}

impl ResolverRPCServer {
    /// Creates a new resolver RPC server with the given resolver.
    pub fn new(resolver: Resolver) -> Self {
        Self {
            resolver: Arc::new(Mutex::new(resolver)),
        }
    }
}

#[tarpc::server]
impl ResolverRPC for ResolverRPCServer {
    async fn lookup_address(self, _: Context, hostname: String) -> Result<AddressResponse, ResolverRPCError> {
        let resolver = self.resolver.lock().await;

        Ok(resolver.lookup_address(hostname).await?)
    }
}
