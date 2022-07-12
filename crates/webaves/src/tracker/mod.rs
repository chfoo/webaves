//! Tracking of quests.
mod table;

/// Manages the quest queue and tracks assignment of quests to fetchers.
pub struct QuestTracker {}

/// General tracker error.
#[derive(thiserror::Error, Debug)]
pub enum TrackerError {
    /// Database error.
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}
