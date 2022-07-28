use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use super::TrackerError;

const APP_ID: i64 = -826887661;

pub struct Table {
    db: Connection,
}

impl Table {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, TrackerError> {
        let exists = path.as_ref().exists();

        let connection = Connection::open(path)?;

        let table = Self { db: connection };

        if exists {
            table.check_app_id()?;
        }

        table.migrate_db()?;

        Ok(table)
    }

    fn check_app_id(&self) -> Result<(), TrackerError> {
        let mut valid = false;
        self.db.pragma_query(None, "application_id", |row| {
            valid = row.get::<_, i64>(0)? == APP_ID;
            Ok(())
        })?;

        if valid {
            Ok(())
        } else {
            todo!()
        }
    }

    fn migrate_db(&self) -> Result<(), TrackerError> {
        let mut version: i64 = 0;
        self.db.pragma_query(None, "user_version", |row| {
            version = row.get(0)?;
            Ok(())
        })?;

        tracing::debug!(version, "migrate_db");

        static DIR: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/migrations/");
        let mut file_versions = Vec::new();

        for file in DIR.files() {
            let filename = file.path().file_name().unwrap().to_str().unwrap();
            let (file_version, _) = filename.split_once('-').unwrap();
            let file_version: i64 = file_version.parse().unwrap();

            file_versions.push((file_version, file));
        }

        file_versions.sort_unstable_by_key(|item| item.0);

        for (file_version, file) in file_versions {
            if file_version > version {
                tracing::info!(version = file_version, "migrate database");
                self.db.execute_batch(file.contents_utf8().unwrap())?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Quest {
    pub id: Uuid,

    /// Processing status.
    pub status: QuestStatus,

    /// Fetch priority.
    ///
    /// More positive numbers are higher priority than numbers towards negative.
    pub priority: i64,

    /// URL of the resource to be fetched.
    pub url: Url,

    /// The previous quest that invoked this quest.
    pub parent: Option<Uuid>,

    /// The ancestry count of the quest.
    ///
    /// A quest with no parent (root) is depth 0, the child is 1,
    /// the grandchild is 2, and so on.
    pub depth: u64,
}

/// Processing status of the quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStatus {
    /// New quest ready to be processed.
    New,

    /// Quest was completed with success.
    Done,

    /// Quest was completed but the resource was not found.
    NotFound,

    /// Quest could not be completed because a network or server error.
    Failed,

    /// Quest could not be completed because of a program error or crash.
    Error,

    /// Quest was previously added but was marked to be ignored.
    Skipped,
}

pub struct Assignment {
    pub id: Uuid,
    pub quest_id: Uuid,
    pub status: AssignmentStatus,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub fetcher_id: Uuid,
}

pub enum AssignmentStatus {
    Active,
    Completed,
    Expired,
    Failed,
}

pub struct AssignmentReport {
    pub assignment_id: Uuid,
    pub message: String,
}


#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;

    #[test]
    fn test_create_db() {
        let dir = TempDir::new("webaves-test").unwrap();
        Table::open(dir.path().join("db")).unwrap();
    }
}
