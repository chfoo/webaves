use std::path::Path;

use rusqlite::Connection;

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
