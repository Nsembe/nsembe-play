//! SQLite persistence. Every SQL statement in the application lives in this module.

pub mod favorites;
pub mod folders;
pub mod home;
pub mod media;
pub mod music;
pub mod playback;
pub mod playlists;
pub mod search;
pub mod settings_store;
pub mod video;

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::errors::Result;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Cheaply clonable handle to the library database.
#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Opens (creating if needed) the database file and applies pending migrations.
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(10));
        let pool = SqlitePoolOptions::new().max_connections(6).connect_with(options).await?;
        MIGRATOR.run(&pool).await?;
        Ok(Db { pool })
    }

    /// In-memory database (tests). A single connection keeps the data alive.
    pub async fn open_in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Db { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// SQL expression: 1 when the given item is a favorite.
pub(crate) fn fav_expr(item_type: &str, id_col: &str) -> String {
    format!("EXISTS(SELECT 1 FROM favorites f WHERE f.item_type = '{item_type}' AND f.item_id = {id_col})")
}

pub(crate) fn clamp_limit(limit: i64) -> i64 {
    limit.clamp(1, 500)
}

/// Builds a `?, ?, ?` placeholder list.
pub(crate) fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}
