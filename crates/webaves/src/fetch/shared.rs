use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{dns::Resolver, service::tracker::QuestTrackerClient};

pub struct InputResources {
    pub dns_resolver: Resolver,
    pub quest_tracker: QuestTrackerClient,
}

#[derive(Clone)]
pub struct SharedResources {
    dns_resolver: Arc<Mutex<Resolver>>,
    quest_tracker: Arc<Mutex<QuestTrackerClient>>,
}

impl SharedResources {
    pub fn new(resources: InputResources) -> Self {
        Self {
            dns_resolver: Arc::new(Mutex::new(resources.dns_resolver)),
            quest_tracker: Arc::new(Mutex::new(resources.quest_tracker)),
        }
    }

    pub fn dns_resolver(&self) -> &Mutex<Resolver> {
        self.dns_resolver.as_ref()
    }

    pub fn quest_tracker(&self) -> &Mutex<QuestTrackerClient> {
        self.quest_tracker.as_ref()
    }
}
