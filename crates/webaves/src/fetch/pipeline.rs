use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use backoff::{backoff::Backoff, ExponentialBackoff};
use tokio::task::{JoinError, JoinSet};

use crate::{error::Error as CrateError, quest::{QuestId, Quest}};

use super::{FetchError, Fetcher, InputResources, SharedResources};

#[derive(Copy, Clone, PartialEq, Eq)]
enum PipelineState {
    Running,
    GracefulShutdown,
}

/// Gets [crate::quest::Quest]s from a [crate::tracker::QuestTracker] and
/// runs [crate::fetch::Fetcher]s.
pub struct Pipeline {
    resources: SharedResources,
    state: PipelineState,
    concurrency: u16,
    tasks: JoinSet<Result<(), FetchError>>,
    task_id_map: HashMap<tokio::task::Id, QuestId>,
    tracker_backoff: ExponentialBackoff,
    tracker_time: Instant,
}

impl Pipeline {
    pub fn new(resources: InputResources) -> Self {
        Self {
            resources: SharedResources::new(resources),
            state: PipelineState::Running,
            concurrency: 0,
            tasks: JoinSet::new(),
            task_id_map: HashMap::new(),
            tracker_backoff: Self::new_tracker_backoff(),
            tracker_time: Instant::now(),
        }
    }

    fn new_tracker_backoff() -> ExponentialBackoff {
        ExponentialBackoff {
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(3600),
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

        let mut quest_tracker = self.resources.quest_tracker().lock().await;

        match quest_tracker.check_out_quest().await? {
            Some(quest) => {
                tracing::info!(quest_id = %quest.id, "quest received");
                self.tracker_backoff.reset();

                let quest_id = quest.id;
                let mut fetcher = Fetcher::new(quest, self.resources.clone());
                let handle = self.tasks.spawn(async move { fetcher.run().await });
                self.task_id_map.insert(handle.id(), quest_id);
            }

            None => {
                tracing::info!("no quest received");
            }
        }

        Ok(())
    }

    async fn process_tasks(&mut self) -> Result<(), CrateError> {
        if let Some(join_result) = self.tasks.join_one_with_id().await {
            match unwrap_finished_task(join_result).await {
                Some((task_id, result)) => {
                    let quest_id = self.task_id_map.remove(&task_id).unwrap();
                    self.process_fetch_result(quest_id, result).await?;
                }
                None => {}
            }
        }

        Ok(())
    }

    async fn process_fetch_result(
        &mut self,
        quest_id: QuestId,
        result: Result<(), FetchError>,
    ) -> Result<(), CrateError> {
        match result {
            Ok(_) => todo!(),
            Err(error) => {
                tracing::error!(%error, "fetch error");

                let mut quest_tracker = self.resources.quest_tracker().lock().await;

                quest_tracker.check_in_quest_error(quest_id, &error).await?;

                Ok(())
            }
        }
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
