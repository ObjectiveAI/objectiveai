//! The lineage row: what the harness itself remembers across runs.
//!
//! Eliza's whole state is the caller's database, written and read by
//! Eliza. The harness remembers only what ties a run to that state
//! and keeps a later run honest against it — one row of one table:
//! the agent id every Eliza id derives from, the room the
//! conversation is, the vector width the memories were built at, and
//! each caller plugin with the version the registry resolved, so a
//! lineage is never resumed against a plugin that changed under it.
//!
//! Read at the top of every run and cached in memory after the first,
//! as cc caches its session id; written BEFORE the runtime starts —
//! on the first run, and whenever the plugin set or the width changed
//! — so a run that dies leaves a row naming the agent id its memories
//! carry.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row as _};

/// The room's external id, constant for the lineage: the room's UUID
/// derives from it and the agent id, so resumption is "same agent id,
/// same room".
pub const ROOM: &str = "diverge";

/// The table.
const CREATE: &str = "CREATE TABLE IF NOT EXISTS eliza_lineage (\
    id smallint PRIMARY KEY CHECK (id = 1), \
    agent_id uuid NOT NULL, \
    room text NOT NULL, \
    embedding_dimensions integer, \
    plugins jsonb NOT NULL)";

/// The row, its uuid and jsonb as text so no driver feature is owed.
const SELECT: &str = "SELECT agent_id::text, room, embedding_dimensions, \
    plugins::text FROM eliza_lineage WHERE id = 1";

/// The row, made or replaced.
const UPSERT: &str = "INSERT INTO eliza_lineage \
    (id, agent_id, room, embedding_dimensions, plugins) \
    VALUES (1, CAST($1 AS uuid), $2, $3, CAST($4 AS jsonb)) \
    ON CONFLICT (id) DO UPDATE SET agent_id = EXCLUDED.agent_id, \
    room = EXCLUDED.room, \
    embedding_dimensions = EXCLUDED.embedding_dimensions, \
    plugins = EXCLUDED.plugins";

/// The lineage, once read: a later run reads no row.
static CACHED: Mutex<Option<Lineage>> = Mutex::new(None);

/// The row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineage {
    /// The agent id, minted on the lineage's first run and pinned into
    /// the runtime constructor after; every Eliza id derives from it.
    pub agent_id: String,
    /// The room's external id: [`ROOM`].
    pub room: String,
    /// The vector width the memories were built at; `None` before any
    /// run named one.
    pub embedding_dimensions: Option<i32>,
    /// Each caller plugin, with the version the registry resolved.
    pub plugins: Vec<Resolved>,
    /// Whether the row exists in the database: `false` for a fresh
    /// lineage until [`save`](Self::save).
    pub persisted: bool,
}

/// One installed plugin, as the row records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolved {
    /// The package name, without a version.
    pub package: String,
    /// The version the registry resolved.
    pub version: String,
}

impl Lineage {
    /// The lineage: the cache after the first look; else the table made
    /// and the row read, or — no row — a fresh lineage with a minted
    /// agent id, not yet persisted. Cached either way, so a fresh
    /// lineage whose save failed keeps its id for the next attempt.
    pub async fn load(pool: &PgPool) -> Result<Self, Error> {
        if let Some(lineage) = cached() {
            return Ok(lineage);
        }
        sqlx::query(CREATE).execute(pool).await?;
        let row = sqlx::query(SELECT).fetch_optional(pool).await?;
        let lineage = match row {
            Some(row) => {
                let plugins: String = row.get(3);
                Lineage {
                    agent_id: row.get(0),
                    room: row.get(1),
                    embedding_dimensions: row.get(2),
                    plugins: serde_json::from_str(&plugins)?,
                    persisted: true,
                }
            }
            None => Lineage {
                agent_id: uuid::Uuid::new_v4().to_string(),
                room: ROOM.to_string(),
                embedding_dimensions: None,
                plugins: Vec::new(),
                persisted: false,
            },
        };
        cache(&lineage);
        Ok(lineage)
    }

    /// Whether this run's facts differ from the row's: not persisted
    /// yet, a different plugin set, or a different width.
    pub fn changed(&self, plugins: &[Resolved], embedding_dimensions: Option<i32>) -> bool {
        !self.persisted
            || self.plugins != plugins
            || self.embedding_dimensions != embedding_dimensions
    }

    /// Write the row with this run's facts, and remember them.
    pub async fn save(
        &mut self,
        pool: &PgPool,
        plugins: Vec<Resolved>,
        embedding_dimensions: Option<i32>,
    ) -> Result<(), Error> {
        let encoded = serde_json::to_string(&plugins)?;
        sqlx::query(UPSERT)
            .bind(&self.agent_id)
            .bind(&self.room)
            .bind(embedding_dimensions)
            .bind(encoded)
            .execute(pool)
            .await?;
        self.plugins = plugins;
        self.embedding_dimensions = embedding_dimensions;
        self.persisted = true;
        cache(self);
        Ok(())
    }
}

fn cached() -> Option<Lineage> {
    CACHED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn cache(lineage: &Lineage) {
    *CACHED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(lineage.clone());
}

/// The row could not be read or written.
#[derive(Debug)]
pub enum Error {
    /// The database would not answer.
    Database(sqlx::Error),
    /// The row's plugin list is not the shape this program writes.
    Plugins(serde_json::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Database(error) => write!(f, "the lineage's database failed: {error}"),
            Error::Plugins(error) => {
                write!(f, "the lineage's plugin list could not be read: {error}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Database(error) => Some(error),
            Error::Plugins(error) => Some(error),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Error::Database(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Plugins(error)
    }
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "lineage",
            "error": self.to_string(),
        })
    }
}
