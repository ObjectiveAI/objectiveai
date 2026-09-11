//! The continuation, in the caller's database: the harvest's frames,
//! one row each.
//!
//! The continuation is three files on disk — `state.db` and the two
//! memory files, [`filesystem::continuation`] — and it leaves the
//! container the way it always has: folded, then read a piece at a
//! time, each piece behind the tag byte naming its file. What changed
//! is where the pieces go. Each is one row of one table, in order,
//! and reading the continuation back is reading the rows in order
//! and appending each to the file its tag names, through the same
//! ingest that used to take them off a socket. Nothing is ever whole
//! in memory: one piece at a time, in and out.
//!
//! # Restored once
//!
//! The files are restored the first time this program runs a loop,
//! and never again: the program is the same system across every run
//! it serves, Hermes appends to the files itself, and the database
//! on disk is the only authority on the session. A later run reads
//! no row and writes no continuation file — it asks the database on
//! disk for the session's tip, which is where the next turn records.
//! Every run still harvests into the rows at its end, replacing them
//! whole in one transaction, so a run that dies mid-harvest leaves
//! the previous rows intact.

use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::{StreamExt as _, TryStreamExt as _};
use sqlx::{PgPool, Row as _};

use crate::filesystem;
use crate::filesystem::continuation::{CheckError, Ingest, IngestError, ReadError};

/// Whether the files have been restored from the rows, this program's
/// life: set once a restore has finished, absent or not.
static RESTORED: AtomicBool = AtomicBool::new(false);

/// The table: the frames, in order.
const CREATE: &str = "CREATE TABLE IF NOT EXISTS continuation (\
    seq integer PRIMARY KEY, frame bytea NOT NULL)";

/// The frames, in order.
const SELECT: &str = "SELECT frame FROM continuation ORDER BY seq";

/// Everything out, before the new frames go in.
const CLEAR: &str = "DELETE FROM continuation";

/// One frame.
const INSERT: &str = "INSERT INTO continuation (seq, frame) VALUES ($1, $2)";

/// The session to resume, with its files on disk: restored from the
/// rows the first time, read from the database on disk after that.
/// `None` is the fresh start — no rows, or a disk with no session.
pub async fn session(pool: &PgPool) -> Result<Option<String>, Error> {
    if RESTORED.load(Ordering::SeqCst) {
        return filesystem::session().await.map_err(Error::Session);
    }
    restore(pool).await
}

/// Read the rows back into the files, and name the session in them.
///
/// The table is made if this is the first run to look. No rows is
/// the fresh start. Otherwise the three files are cleared — a restore
/// that failed partway must not append onto its own leavings — and
/// each row's frame is appended to the file its tag names, one at a
/// time; then the delivered database is proved to open and asked for
/// its most recently active session, exactly as the socket's
/// delivery used to be. Restored is recorded only on success, so a
/// failure is retried by the next run.
async fn restore(pool: &PgPool) -> Result<Option<String>, Error> {
    sqlx::query(CREATE).execute(pool).await?;
    let mut rows = sqlx::query(SELECT).fetch(pool);
    let mut ingest: Option<Ingest> = None;
    while let Some(row) = rows.try_next().await? {
        let frame: Vec<u8> = row.get(0);
        if ingest.is_none() {
            filesystem::continuation::clear().await.map_err(Error::Clear)?;
            ingest = Some(Ingest::start().await.map_err(IngestError::from)?);
        }
        ingest
            .as_mut()
            .expect("started just above")
            .push(&frame)
            .await?;
    }
    let session = match ingest {
        None => None,
        Some(ingest) => {
            ingest.finish().await?;
            Some(filesystem::continuation::check().await?)
        }
    };
    RESTORED.store(true, Ordering::SeqCst);
    Ok(session)
}

/// Replace the rows with the files as the run left them: the
/// database folded, then every piece of every file, in order, in one
/// transaction.
pub async fn harvest(pool: &PgPool) -> Result<(), Error> {
    let mut frames = Box::pin(filesystem::continuation::stream().await?);
    let mut transaction = pool.begin().await?;
    sqlx::query(CREATE).execute(&mut *transaction).await?;
    sqlx::query(CLEAR).execute(&mut *transaction).await?;
    let mut seq: i32 = 0;
    while let Some(frame) = frames.next().await {
        let frame = frame?;
        sqlx::query(INSERT)
            .bind(seq)
            .bind(frame)
            .execute(&mut *transaction)
            .await?;
        seq += 1;
    }
    transaction.commit().await?;
    Ok(())
}

/// The continuation could not be restored or harvested.
#[derive(Debug)]
pub enum Error {
    /// The database would not answer.
    Database(sqlx::Error),
    /// A row's frame could not be landed in its file.
    Ingest(IngestError),
    /// The restored database did not check out.
    Check(CheckError),
    /// The files could not be cleared for a restore.
    Clear(std::io::Error),
    /// The files could not be folded and read for the harvest.
    Read(ReadError),
    /// The session's tip could not be read from the database on disk.
    Session(sqlx::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Database(error) => {
                write!(f, "the continuation's database failed: {error}")
            }
            Error::Ingest(error) => write!(f, "{error}"),
            Error::Check(error) => write!(f, "{error}"),
            Error::Clear(error) => {
                write!(f, "the continuation's files could not be cleared: {error}")
            }
            Error::Read(error) => write!(f, "{error}"),
            Error::Session(error) => {
                write!(f, "the session could not be read from state.db: {error}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Database(error) | Error::Session(error) => Some(error),
            Error::Ingest(error) => Some(error),
            Error::Check(error) => Some(error),
            Error::Clear(error) => Some(error),
            Error::Read(error) => Some(error),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Error::Database(error)
    }
}

impl From<IngestError> for Error {
    fn from(error: IngestError) -> Self {
        Error::Ingest(error)
    }
}

impl From<CheckError> for Error {
    fn from(error: CheckError) -> Self {
        Error::Check(error)
    }
}

impl From<ReadError> for Error {
    fn from(error: ReadError) -> Self {
        Error::Read(error)
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
