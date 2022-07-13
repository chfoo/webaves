use std::sync::{Arc, Mutex};

use tarpc::{client::RpcError, context::Context};

use crate::{error::Error as CrateError, quest::QuestId, tracker::QuestTracker};
use crate::{quest::Quest, retry::Retry};

pub const SERVICE_NAME: &str = "quest-tracker";

#[tarpc::service]
pub trait QuestTrackerRPC {
    async fn check_out_quest() -> Option<Quest>;
    async fn check_in_quest_error(quest_id: QuestId, message: String) -> Option<Quest>;
}

pub struct QuestTrackerRPCServer {
    inner: Arc<Mutex<QuestTracker>>,
}

impl QuestTrackerRPCServer {
    pub fn new(inner: QuestTracker) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

#[tarpc::server]
impl QuestTrackerRPC for QuestTrackerRPCServer {
    async fn check_out_quest(self, _: Context) -> Option<Quest> {
        todo!()
    }
    async fn check_in_quest_error(
        self,
        _: Context,
        quest_id: QuestId,
        message: String,
    ) -> Option<Quest> {
        todo!()
    }
}

/// Facade to [QuestTrackerRPCClient].
pub struct QuestTrackerClient {
    inner: QuestTrackerRPCClient,
    retry: Retry,
}

impl QuestTrackerClient {
    /// Creates a new `QuestTrackerClient`.
    pub fn new(inner: QuestTrackerRPCClient) -> Self {
        Self {
            inner,
            retry: Retry::default(),
        }
    }

    fn check_retry_success<T>(result: &Result<T, RpcError>) -> bool {
        match result {
            Ok(_) => true,
            Err(error) => {
                tracing::error!(%error, "tracker request RPC error");

                // TODO: determine if Disconnected is fatal

                false
            }
        }
    }

    /// Facade to [QuestTrackerRPCClient::check_out_quest].
    pub async fn check_out_quest(&mut self) -> Result<Option<Quest>, CrateError> {
        Ok(self
            .retry
            .async_run(
                || self.inner.check_out_quest(Context::current()),
                Self::check_retry_success,
            )
            .await
            .unwrap())
    }

    /// Facade to [QuestTrackerRPCClient::check_in_quest_error].
    pub async fn check_in_quest_error(
        &mut self,
        quest_id: QuestId,
        error: &dyn std::error::Error,
    ) -> Result<(), CrateError> {
        todo!()
    }
}
