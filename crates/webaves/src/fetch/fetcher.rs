use crate::quest::Quest;

use super::SharedResources;

pub struct Fetcher {
    quest: Quest,
    shared_data: SharedResources,
}

impl Fetcher {
    pub fn new(quest: Quest, shared_data: SharedResources) -> Self {
        Self {
            quest,
            shared_data,
        }
    }

    #[tracing::instrument(skip_all, level = "info", name = "fetcher", fields(quest_id = %self.quest.id))]
    pub async fn run(&mut self) -> Result<(), FetchError> {
        match self.quest.url.scheme() {
            "http" | "https" => {
                todo!()
            }
            _ => Err(FetchError::UnsupportedScheme(
                self.quest.url.scheme().to_string(),
            )),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("unsupported scheme {0}")]
    UnsupportedScheme(String),
}
