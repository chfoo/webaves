//! DNS resolution service.

use std::sync::Arc;

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

    fn log_resolver_error(&self, error: &ResolverError) {
        match error {
            ResolverError::Protocol(error) => tracing::error!(%error, "lookup address failed"),
            ResolverError::Io(error) => tracing::error!(%error, "lookup address failed"),
            _ => {}
        }
    }
}

#[tarpc::server]
impl ResolverRPC for ResolverRPCServer {
    async fn lookup_address(
        self,
        _: Context,
        hostname: String,
    ) -> Result<AddressResponse, ResolverRPCError> {
        let resolver = self.resolver.lock().await;

        let result = resolver.lookup_address(hostname).await;

        if let Err(error) = &result {
            self.log_resolver_error(error);
        }

        Ok(result?)
    }
}

/// Errors returned from the DNS resolver RPC service.
#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize)]
pub enum ResolverRPCError {
    /// Non-existent domain.
    #[error("non-existent domain")]
    NoName(String),

    /// No records for given record type.
    #[error("no records for given record type")]
    NoRecord(String),

    /// Other negative response.
    #[error("negative response")]
    Negative(String),

    /// Protocol error
    #[error("protocol error")]
    Protocol(String),

    /// Invalid argument
    #[error("invalid argument")]
    InvalidArg(String),

    /// Other error
    #[error("error")]
    Other(String),
}

impl From<ResolverError> for ResolverRPCError {
    fn from(error: ResolverError) -> Self {
        match error {
            ResolverError::NoName(error) => Self::NoName(crate::error::format_to_string(error)),
            ResolverError::NoRecord(error) => Self::NoRecord(crate::error::format_to_string(error)),
            ResolverError::Negative(error) => Self::Negative(crate::error::format_to_string(error)),
            ResolverError::Protocol(error) => Self::Protocol(crate::error::format_to_string(error)),
            ResolverError::InvalidArg(error) => {
                Self::InvalidArg(crate::error::format_to_string(error.as_ref()))
            }
            ResolverError::Io(error) => Self::Other(crate::error::format_to_string(error)),
        }
    }
}
