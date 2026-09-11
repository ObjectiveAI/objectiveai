//! The continuation: the thread, its usage baseline, and the rollout
//! files, in the caller's database.
//!
//! Codex keeps a conversation as a THREAD: append-only JSONL rollouts
//! under `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<ts>-<thread_id>
//! [_<n>].jsonl` (`.zst` once compressed; `archived_sessions/` once
//! archived), and `codex exec resume <thread_id>` finds them by
//! scanning the tree when its SQLite index has no entry — so the
//! files ARE the continuation, and the index is Codex's to rebuild.
//! Nothing else travels: not the state database, not the display
//! history, not the config and the login, which are the harness's
//! and the vault's.
//!
//! Two tables, reached through the proxy's loopback pgwire: the
//! thread row — the id `exec resume` takes, and the last cumulative
//! usage `turn.completed` reported, which is the baseline the next
//! delta is billed from — and the files, one row each, bytes
//! verbatim, keyed by their path under the home.
//!
//! # Restored once, harvested every run
//!
//! The files are written under the home the first time this program
//! runs a loop, and never again: Codex appends to them itself, and
//! the disk is the authority from then on. The thread row is read
//! once and cached. Every run harvests at its end — the thread's
//! files as they are, replacing the rows whole in one transaction
//! beside the row — so a run that dies mid-harvest leaves the
//! previous rows intact.

use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::TryStreamExt as _;
use sqlx::{PgPool, Row as _};

use crate::auth::CODEX_HOME;
use crate::response::Usage;

/// The subdirectories a thread's rollouts may be under.
const ROLLOUT_DIRS: [&str; 2] = ["sessions", "archived_sessions"];

/// Whether the files have been restored from the rows, this program's
/// life.
static RESTORED: AtomicBool = AtomicBool::new(false);

/// The thread, once read.
static CACHED: Mutex<Option<Thread>> = Mutex::new(None);

const CREATE_THREAD: &str = "CREATE TABLE IF NOT EXISTS codex_thread (\
    id smallint PRIMARY KEY CHECK (id = 1), \
    thread_id text NOT NULL, \
    usage jsonb NOT NULL)";
const CREATE_FILES: &str = "CREATE TABLE IF NOT EXISTS codex_files (\
    path text PRIMARY KEY, content bytea NOT NULL)";
const SELECT_THREAD: &str = "SELECT thread_id, usage::text FROM codex_thread WHERE id = 1";
const SELECT_FILES: &str = "SELECT path, content FROM codex_files ORDER BY path";
const UPSERT_THREAD: &str = "INSERT INTO codex_thread (id, thread_id, usage) \
    VALUES (1, $1, CAST($2 AS jsonb)) \
    ON CONFLICT (id) DO UPDATE SET thread_id = EXCLUDED.thread_id, \
    usage = EXCLUDED.usage";
const CLEAR_FILES: &str = "DELETE FROM codex_files";
const INSERT_FILE: &str = "INSERT INTO codex_files (path, content) VALUES ($1, $2)";

/// The thread the lineage is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    /// The id `codex exec resume` takes; `None` before any turn ran.
    pub thread_id: Option<String>,
    /// The last cumulative usage `turn.completed` reported: the
    /// baseline the next turn's delta is billed from.
    pub usage: Usage,
}

/// The thread: cached after the first look; else the tables made,
/// the row read, and — the first time this program looks — the files
/// restored under the home. No row is the fresh start.
pub async fn load(pool: &PgPool) -> Result<Thread, Error> {
    if let Some(thread) = cached() {
        return Ok(thread);
    }
    sqlx::query(CREATE_THREAD).execute(pool).await?;
    sqlx::query(CREATE_FILES).execute(pool).await?;
    let row = sqlx::query(SELECT_THREAD).fetch_optional(pool).await?;
    let thread = match row {
        Some(row) => {
            let usage: String = row.get(1);
            Thread {
                thread_id: Some(row.get(0)),
                usage: serde_json::from_str(&usage)?,
            }
        }
        None => Thread {
            thread_id: None,
            usage: Usage::default(),
        },
    };
    if !RESTORED.load(Ordering::SeqCst) {
        restore(pool).await?;
        RESTORED.store(true, Ordering::SeqCst);
    }
    cache(&thread);
    Ok(thread)
}

/// Write every file row under the home, parents made. A path that is
/// absolute, names a root, or contains `..` is refused: the rows name
/// files inside the home and do not get to escape it.
async fn restore(pool: &PgPool) -> Result<(), Error> {
    let mut rows = sqlx::query(SELECT_FILES).fetch(pool);
    while let Some(row) = rows.try_next().await? {
        let path: String = row.get(0);
        let content: Vec<u8> = row.get(1);
        let relative = Path::new(&path);
        if !relative
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
        {
            return Err(Error::Escape(path));
        }
        let target = Path::new(CODEX_HOME).join(relative);
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(Error::Restore)?;
        }
        tokio::fs::write(&target, &content).await.map_err(Error::Restore)?;
    }
    Ok(())
}

/// Replace the rows with the thread and its files as the run left
/// them, in one transaction, and remember the thread.
pub async fn harvest(pool: &PgPool, thread: &Thread) -> Result<(), Error> {
    let Some(thread_id) = &thread.thread_id else {
        return Err(Error::NoThread);
    };
    let files = rollouts(thread_id).await.map_err(Error::Harvest)?;
    let usage = serde_json::to_string(&thread.usage)?;
    let mut transaction = pool.begin().await?;
    sqlx::query(CREATE_THREAD).execute(&mut *transaction).await?;
    sqlx::query(CREATE_FILES).execute(&mut *transaction).await?;
    sqlx::query(UPSERT_THREAD)
        .bind(thread_id)
        .bind(usage)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(CLEAR_FILES).execute(&mut *transaction).await?;
    for (path, content) in files {
        sqlx::query(INSERT_FILE)
            .bind(path)
            .bind(content)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    cache(thread);
    Ok(())
}

/// Every rollout of the thread, by path under the home: the files
/// under the rollout directories whose name carries the thread id,
/// read as bytes.
async fn rollouts(thread_id: &str) -> io::Result<Vec<(String, Vec<u8>)>> {
    let home = Path::new(CODEX_HOME);
    let mut files = Vec::new();
    for dir in ROLLOUT_DIRS {
        let root = home.join(dir);
        if !tokio::fs::try_exists(&root).await? {
            continue;
        }
        let mut pending: Vec<PathBuf> = vec![root];
        while let Some(directory) = pending.pop() {
            let mut entries = tokio::fs::read_dir(&directory).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if entry.file_type().await?.is_dir() {
                    pending.push(path);
                    continue;
                }
                let name = entry.file_name();
                if !name.to_string_lossy().contains(thread_id) {
                    continue;
                }
                let relative = path
                    .strip_prefix(home)
                    .map_err(|_| io::Error::other("a rollout is outside the home"))?;
                let relative = relative
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                files.push((relative, tokio::fs::read(&path).await?));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

fn cached() -> Option<Thread> {
    CACHED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn cache(thread: &Thread) {
    *CACHED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(thread.clone());
}

/// The continuation could not be restored or harvested.
#[derive(Debug)]
pub enum Error {
    /// The database would not answer.
    Database(sqlx::Error),
    /// The thread row's usage is not the shape this program writes.
    Usage(serde_json::Error),
    /// A file row names a path outside the home.
    Escape(String),
    /// A file could not be written under the home.
    Restore(io::Error),
    /// The rollouts could not be read.
    Harvest(io::Error),
    /// No turn ever named a thread, so there is nothing to harvest.
    NoThread,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Database(error) => write!(f, "the continuation's database failed: {error}"),
            Error::Usage(error) => write!(f, "the thread row's usage could not be read: {error}"),
            Error::Escape(path) => write!(f, "a continuation file escapes the home: {path}"),
            Error::Restore(error) => write!(f, "a rollout could not be restored: {error}"),
            Error::Harvest(error) => write!(f, "the rollouts could not be read: {error}"),
            Error::NoThread => write!(f, "no turn ever named a thread"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Database(error) => Some(error),
            Error::Usage(error) => Some(error),
            Error::Restore(error) | Error::Harvest(error) => Some(error),
            Error::Escape(_) | Error::NoThread => None,
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
        Error::Usage(error)
    }
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "continuation",
            "error": self.to_string(),
        })
    }
}
