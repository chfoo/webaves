//! Tracking of quests.

use std::path::Path;
mod table;

/// Manages the quest queue and tracks assignment of quests to fetchers.
pub struct QuestTracker {
    table: table::Table,
}

impl QuestTracker {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, TrackerError> {
        let table = table::Table::open(path)?;

        Ok(Self { table })
    }
}

/// General tracker error.
#[derive(thiserror::Error, Debug)]
pub enum TrackerError {
    /// Database error.
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}
