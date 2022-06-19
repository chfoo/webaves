//! Echo test service.

use tarpc::context::Context;

/// Name of the service.
pub const SERVICE_NAME: &str = "echo";

/// Echo service for testing.
#[tarpc::service]
pub trait EchoRPC {
    /// Return the given text unchanged.
    async fn echo(name: String) -> String;
}

/// Echo server.
#[derive(Clone)]
pub struct EchoRPCServer;

#[tarpc::server]
impl EchoRPC for EchoRPCServer {
    async fn echo(self, _: Context, text: String) -> String {
        text
    }
}
