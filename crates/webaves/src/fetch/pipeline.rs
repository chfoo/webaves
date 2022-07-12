use std::time::{Duration, Instant};

use backoff::{backoff::Backoff, ExponentialBackoff};
use tarpc::context::Context;
use tokio::{
    sync::mpsc,
    task::{JoinError, JoinSet},
};

use crate::error::Error as CrateError;
use crate::{dns::Resolver, service::tracker::QuestTrackerRPCClient};

use super::{FetchError, Fetcher, ResolverRequest};

#[derive(Copy, Clone, PartialEq, Eq)]
enum PipelineState {
    Running,
    GracefulShutdown,
}

/// Gets [crate::quest::Quest]s from a [crate::tracker::QuestTracker] and runs [crate::fetch::Fetcher]s.
pub struct Pipeline {
    quest_tracker: QuestTrackerRPCClient,
    resolver: Resolver,
    resolver_request_receiver: mpsc::Receiver<ResolverRequest>,
    resolver_request_sender: mpsc::Sender<ResolverRequest>,

    state: PipelineState,
    concurrency: u16,
    tasks: JoinSet<Result<(), FetchError>>,
    tracker_backoff: ExponentialBackoff,
    tracker_time: Instant,
}

impl Pipeline {
    pub fn new(quest_tracker: QuestTrackerRPCClient, dns_resolver: Resolver) -> Self {
        let (dns_resolver_request_sender, dns_resolver_request_receiver) = mpsc::channel(1);

        Self {
            quest_tracker,
            resolver: dns_resolver,
            resolver_request_receiver: dns_resolver_request_receiver,
            resolver_request_sender: dns_resolver_request_sender,
            state: PipelineState::Running,
            concurrency: 0,
            tasks: JoinSet::new(),
            tracker_backoff: Self::new_backoff(),
            tracker_time: Instant::now(),
        }
    }

    fn new_backoff() -> ExponentialBackoff {
        ExponentialBackoff {
            initial_interval: Duration::from_secs(2),
            max_interval: Duration::from_secs(3600),
            max_elapsed_time: Some(Duration::from_secs(3600 * 24)),
            ..Default::default()
        }
    }

    pub async fn run(&mut self) -> Result<(), CrateError> {
        self.state = PipelineState::Running;

        loop {
            if !self.run_once().await? {
                break;
            }
        }

        Ok(())
    }

    async fn run_once(&mut self) -> Result<bool, CrateError> {
        tracing::trace!(
            concurrency = self.concurrency,
            tasks_len = self.tasks.len(),
            "run loop"
        );

        let backoff_duration = self
            .tracker_backoff
            .next_backoff()
            .unwrap_or(self.tracker_backoff.max_interval);

        match self.state {
            PipelineState::Running => {
                if self.concurrency < self.tasks.len() as u16
                    && self.tracker_time.elapsed() >= backoff_duration
                {
                    self.request_quest().await?;
                    self.tracker_time = Instant::now();
                }
            }
            PipelineState::GracefulShutdown => {
                if self.tasks.is_empty() {
                    return Ok(false);
                }
            }
        }

        tokio::select! {
            _ = self.process_tasks() => {}
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
        };

        Ok(true)
    }

    async fn request_quest(&mut self) -> Result<(), CrateError> {
        tracing::info!("requesting quest");

        match self.quest_tracker.check_out_quest(Context::current()).await {
            Ok(result) => match result {
                Some(quest) => {
                    tracing::info!(id = quest.id, "quest received");
                    self.tracker_backoff.reset();

                    let mut fetcher = Fetcher::new(quest, self.resolver_request_sender.clone());
                    self.tasks.spawn(async move { fetcher.run().await });
                }

                None => {
                    tracing::info!("no quest received");
                }
            },
            Err(error) => {
                tracing::error!(%error, "tracker request RPC error");
                // TODO: determine whether if any error is fatal here
            }
        }

        Ok(())
    }

    async fn process_tasks(&mut self) -> Result<(), CrateError> {
        if let Some(join_result) = self.tasks.join_one().await {
            match unwrap_finished_task(join_result).await {
                Some(result) => {
                    self.process_fetch_result(result).await?;
                }
                None => {}
            }
        }

        Ok(())
    }

    async fn process_fetch_result(
        &mut self,
        result: Result<(), FetchError>,
    ) -> Result<(), CrateError> {
        todo!()
    }
}

async fn unwrap_finished_task<T>(join_result: Result<T, JoinError>) -> Option<T> {
    match join_result {
        Ok(result) => Some(result),
        Err(error) => {
            if error.is_panic() {
                std::panic::resume_unwind(error.into_panic())
            }
            if error.is_cancelled() {
                unimplemented!()
            }

            None
        }
    }
}
