use tokio::sync::oneshot;

use crate::dns::{AddressResponse, ResolverError};

pub type ResolverResult = Result<AddressResponse, ResolverError>;

pub struct ResolverRequest {
    hostname: String,
    sender: oneshot::Sender<ResolverResult>,
}

impl ResolverRequest {
    fn new(hostname: String) -> (Self, oneshot::Receiver<ResolverResult>) {
        let (sender, receiver) = oneshot::channel();

        (Self { hostname, sender }, receiver)
    }
}
