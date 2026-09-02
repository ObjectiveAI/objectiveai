//! What a Hermes continuation holds: the session store and the two
//! memory files.
//!
//! Hermes keeps a conversation in `$HERMES_HOME/state.db` — a SQLite
//! database in WAL mode, schema version 26 and still moving — and
//! what it has learned across conversations in two capped Markdown
//! files, `memories/MEMORY.md` and `memories/USER.md`, injected into
//! the system prompt as a frozen snapshot at session start. Those
//! three files are the continuation. Nothing else under the Hermes
//! home travels: configuration is rendered from the request each
//! run, `auth.json` entries are resources, caches regenerate, skill
//! writing is unsupported, and the gateway-only machinery (cron,
//! kanban, platform ledgers) never runs here.
//!
//! The database goes WHOLE rather than as extracted rows: its
//! gateway-only tables are empty in this container, and what the
//! resume needs is spread across `sessions`, `messages` (with their
//! compaction flags), the `system_prompts` row a session restores
//! verbatim, and the `state_meta` keys goals live under — a moving
//! target better copied than re-inserted.
//!
//! # On the wire: tagged chunks
//!
//! The protocol keeps a continuation's chunks apart, boundaries and
//! order intact, from this container's closer to the caller and
//! back. So each chunk leads with one tag byte naming its file —
//! `0` state.db, `1` MEMORY.md, `2` USER.md — and carries a piece of
//! that file. A file longer than the chunk ceiling spans several
//! consecutive chunks with the same tag; a present-but-empty file is
//! one tag-only chunk; the files come in ascending tag order. No
//! lengths, no envelope: the boundaries the protocol preserves are
//! the framing.
//!
//! # Two standing rules
//!
//! The harvest happens after the gateway process has EXITED, and
//! folds the write-ahead log into the main file itself: Hermes's own
//! close runs only a passive checkpoint, so a WAL survives even a
//! clean exit, and a database separated from its WAL loses committed
//! transactions. And the configuration the harness writes must never
//! enable Hermes's session retention pruning, which would silently
//! delete the older part of a lineage from inside the continuation.

use std::error;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use diverge_provider_sdk::endpoints::agentic_loop::run::client::channel_response::CHUNK_SIZE;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
use sqlx::{ConnectOptions as _, Connection as _, Executor as _, Row as _};

/// Where Hermes keeps its state, fixed for the container's life: the
/// default home for the container's root user, with no
/// `HERMES_HOME` override — so what [`Continuation::write`] lays
/// down is what the gateway finds, and what the gateway leaves is
/// what [`Continuation::read`] sweeps up.
pub const HERMES_HOME: &str = "/root/.hermes";

/// The session store, at the home's root.
const STATE_DB: &str = "state.db";

/// The memory directory under the home.
const MEMORIES: &str = "memories";

/// The agent's own notes, capped by Hermes at 2,200 characters.
const MEMORY_FILE: &str = "MEMORY.md";

/// The user profile, capped by Hermes at 1,375 characters.
const USER_FILE: &str = "USER.md";

/// Tag for chunks of `state.db`.
const STATE_DB_TAG: u8 = 0;

/// Tag for chunks of `memories/MEMORY.md`.
const MEMORY_TAG: u8 = 1;

/// Tag for chunks of `memories/USER.md`.
const USER_TAG: u8 = 2;

/// How much of a file one chunk carries: the protocol's ceiling less
/// the tag.
const PAYLOAD: usize = CHUNK_SIZE - 1;

/// A continuation, opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Continuation {
    /// `state.db`, whole: a self-contained SQLite file, its WAL
    /// folded in. Every continuation has one — a run that produced
    /// no database produced no continuation.
    pub state_db: Vec<u8>,
    /// `memories/MEMORY.md`, when the agent has written one.
    pub memory: Option<Vec<u8>>,
    /// `memories/USER.md`, when the agent has written one.
    pub user: Option<Vec<u8>>,
}

impl Continuation {
    /// Open the chunks the server delivered: each led by its tag,
    /// gathered by file.
    ///
    /// The rules, each with its refusal: a chunk with no bytes at
    /// all has no tag ([`ContinuationError::Empty`]); a tag that is
    /// none of the three ([`UnknownTag`](ContinuationError::UnknownTag));
    /// a file's chunks must be contiguous and the files in ascending
    /// tag order, so a tag lower than the last seen is a sequence
    /// that was reordered or interleaved
    /// ([`Order`](ContinuationError::Order)); and a continuation
    /// with no `state.db` is no continuation
    /// ([`MissingStateDb`](ContinuationError::MissingStateDb)).
    pub fn parse(chunks: &[Vec<u8>]) -> Result<Self, ContinuationError> {
        let mut state_db: Option<Vec<u8>> = None;
        let mut memory: Option<Vec<u8>> = None;
        let mut user: Option<Vec<u8>> = None;
        let mut last: Option<u8> = None;
        for chunk in chunks {
            let (tag, payload) =
                chunk.split_first().ok_or(ContinuationError::Empty)?;
            let file = match *tag {
                STATE_DB_TAG => &mut state_db,
                MEMORY_TAG => &mut memory,
                USER_TAG => &mut user,
                tag => return Err(ContinuationError::UnknownTag(tag)),
            };
            if let Some(after) = last
                && *tag < after
            {
                return Err(ContinuationError::Order { tag: *tag, after });
            }
            last = Some(*tag);
            file.get_or_insert_with(Vec::new).extend_from_slice(payload);
        }
        Ok(Continuation {
            state_db: state_db.ok_or(ContinuationError::MissingStateDb)?,
            memory,
            user,
        })
    }

    /// The state as the chunks the run closes with —
    /// [`parse`](Self::parse)'s exact inverse. Cannot fail: bytes
    /// are cut and tagged, nothing more.
    pub fn tokenize(&self) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();
        push_file(&mut chunks, STATE_DB_TAG, &self.state_db);
        if let Some(memory) = &self.memory {
            push_file(&mut chunks, MEMORY_TAG, memory);
        }
        if let Some(user) = &self.user {
            push_file(&mut chunks, USER_TAG, user);
        }
        chunks
    }

    /// Lay the files down under [`HERMES_HOME`], and prove the
    /// database opens. Called before `hermes gateway` starts.
    ///
    /// Whatever a stale `state.db` family is doing there goes first
    /// — a leftover write-ahead log beside a fresh copy would be
    /// replayed INTO it — then the three files are written, and then
    /// the database is opened and asked `PRAGMA quick_check`. That
    /// last step is not caution for its own sake: Hermes HEALS a
    /// database it cannot open, by quarantining it and starting
    /// fresh, and a run that started fresh would harvest an amnesiac
    /// continuation over the lineage without anyone noticing. So a
    /// delivery that does not check out fails loudly here instead
    /// ([`WriteError::Corrupt`]). `quick_check` is one sequential
    /// read of the file, once per run.
    ///
    /// A fresh start writes nothing — Hermes creates its own schema —
    /// so this is only ever called with a delivered continuation.
    pub async fn write(&self) -> Result<(), WriteError> {
        let home = Path::new(HERMES_HOME);
        let memories = home.join(MEMORIES);
        tokio::fs::create_dir_all(&memories).await?;

        let db = home.join(STATE_DB);
        remove_if_present(&db).await?;
        for suffix in ["-wal", "-shm", "-journal"] {
            remove_if_present(&sidecar(&db, suffix)).await?;
        }
        tokio::fs::write(&db, &self.state_db).await?;
        if let Some(memory) = &self.memory {
            tokio::fs::write(memories.join(MEMORY_FILE), memory).await?;
        }
        if let Some(user) = &self.user {
            tokio::fs::write(memories.join(USER_FILE), user).await?;
        }

        let mut connection = open(&db).await?;
        let verdict = sqlx::query("PRAGMA quick_check")
            .persistent(false)
            .fetch_all(&mut connection)
            .await?
            .iter()
            .map(|row| row.get::<String, _>(0))
            .collect::<Vec<_>>();
        connection.close().await?;
        if verdict != ["ok"] {
            return Err(WriteError::Corrupt(verdict));
        }
        Ok(())
    }

    /// Harvest the state under [`HERMES_HOME`] into a
    /// [`Continuation`]. Called after the gateway process has
    /// exited — nothing else may hold the database.
    ///
    /// The database is folded before it is read: `VACUUM` first,
    /// which rewrites every page (and reads every page, so damage
    /// surfaces here) THROUGH the write-ahead log, then a
    /// `wal_checkpoint(TRUNCATE)`, which moves the log's frames into
    /// the main file and empties the log; then the connection is
    /// closed, which — being the last — deletes the log and the
    /// shared-memory file. A checkpoint that could not complete
    /// means something still holds the database
    /// ([`ReadError::Busy`]); a log still carrying bytes after the
    /// close means the fold did not happen
    /// ([`ReadError::WalRemains`]). Only then is `state.db` read —
    /// a single file that is the whole database. `VACUUM` needs up
    /// to twice the file's size free under the home while it runs.
    ///
    /// The memory files are read as they are; absence is not an
    /// error, it is a run that never wrote one.
    pub async fn read() -> Result<Self, ReadError> {
        let home = Path::new(HERMES_HOME);
        let db = home.join(STATE_DB);
        fold(&db).await?;
        let state_db = tokio::fs::read(&db).await?;
        let memories = home.join(MEMORIES);
        let memory = read_optional(memories.join(MEMORY_FILE)).await?;
        let user = read_optional(memories.join(USER_FILE)).await?;
        Ok(Continuation {
            state_db,
            memory,
            user,
        })
    }
}

/// Cut one file into tagged chunks: pieces of at most [`PAYLOAD`]
/// bytes, each behind its tag; an empty file is one tag-only chunk,
/// so presence survives the trip.
fn push_file(chunks: &mut Vec<Vec<u8>>, tag: u8, bytes: &[u8]) {
    if bytes.is_empty() {
        chunks.push(vec![tag]);
        return;
    }
    for piece in bytes.chunks(PAYLOAD) {
        let mut chunk = Vec::with_capacity(piece.len() + 1);
        chunk.push(tag);
        chunk.extend_from_slice(piece);
        chunks.push(chunk);
    }
}

/// Fold the write-ahead log into `state.db` and leave a lone file:
/// see [`Continuation::read`].
async fn fold(db: &Path) -> Result<(), ReadError> {
    let mut connection = open(db).await?;
    // Autocommit: nothing here opened a transaction, and VACUUM
    // refuses to run inside one.
    connection.execute("VACUUM").await?;
    let row = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .persistent(false)
        .fetch_one(&mut connection)
        .await?;
    // Column 0 is 1 when the checkpoint was blocked from completing.
    let blocked = row.get::<i64, _>(0);
    if blocked != 0 {
        // The close's own failure is beside the point now.
        let _ = connection.close().await;
        return Err(ReadError::Busy);
    }
    connection.close().await?;

    let wal = sidecar(db, "-wal");
    match tokio::fs::metadata(&wal).await {
        Ok(meta) if meta.len() > 0 => {
            return Err(ReadError::WalRemains(meta.len()));
        }
        Ok(_) => tokio::fs::remove_file(&wal).await?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    remove_if_present(&sidecar(db, "-shm")).await?;
    Ok(())
}

/// One connection to the database, read/write, never creating: a
/// missing file is an error, not a fresh start. Never a pool — a
/// pool's idle connections would keep the last-close cleanup from
/// ever firing. Nothing else is set: no journal mode (the file's
/// own stays), no optimize-on-close (that would write through the
/// log after the fold).
async fn open(db: &Path) -> Result<SqliteConnection, sqlx::Error> {
    SqliteConnectOptions::new()
        .filename(db)
        .create_if_missing(false)
        .read_only(false)
        .connect()
        .await
}

/// A SQLite sidecar's path: the database's own name with the suffix
/// appended, `state.db-wal` for `state.db`.
fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = db.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Remove a file that may not exist; absence is fine.
async fn remove_if_present(path: &Path) -> io::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Read a file that may not exist; absence is `None`.
async fn read_optional(path: PathBuf) -> io::Result<Option<Vec<u8>>> {
    match tokio::fs::read(&path).await {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// A delivered continuation that could not be opened.
#[derive(Debug)]
pub enum ContinuationError {
    /// A chunk with no bytes at all, so not even a tag.
    Empty,
    /// A tag that names none of the three files.
    UnknownTag(u8),
    /// A tag lower than the one before it: the files come in
    /// ascending order, each contiguous, and this sequence was
    /// reordered or interleaved.
    Order {
        /// The tag that arrived.
        tag: u8,
        /// The tag it arrived after.
        after: u8,
    },
    /// No `state.db` chunk at all.
    MissingStateDb,
}

impl fmt::Display for ContinuationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContinuationError::Empty => {
                f.write_str("a continuation chunk is empty")
            }
            ContinuationError::UnknownTag(tag) => {
                write!(f, "unknown continuation chunk tag {tag}")
            }
            ContinuationError::Order { tag, after } => write!(
                f,
                "continuation chunk tag {tag} arrived after tag {after}"
            ),
            ContinuationError::MissingStateDb => {
                f.write_str("the continuation carries no state.db")
            }
        }
    }
}

impl error::Error for ContinuationError {}

/// A continuation that could not be laid down.
#[derive(Debug)]
pub enum WriteError {
    /// A file could not be written, or the stale family removed.
    Io(io::Error),
    /// The database could not be opened or checked.
    Sqlite(sqlx::Error),
    /// The database opened but did not check out: `quick_check`'s
    /// findings, verbatim.
    Corrupt(Vec<String>),
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::Io(error) => {
                write!(f, "the continuation could not be written: {error}")
            }
            WriteError::Sqlite(error) => {
                write!(f, "the delivered state.db could not be opened: {error}")
            }
            WriteError::Corrupt(findings) => write!(
                f,
                "the delivered state.db is corrupt: {}",
                findings.join("; ")
            ),
        }
    }
}

impl error::Error for WriteError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            WriteError::Io(error) => Some(error),
            WriteError::Sqlite(error) => Some(error),
            WriteError::Corrupt(_) => None,
        }
    }
}

impl From<io::Error> for WriteError {
    fn from(error: io::Error) -> Self {
        WriteError::Io(error)
    }
}

impl From<sqlx::Error> for WriteError {
    fn from(error: sqlx::Error) -> Self {
        WriteError::Sqlite(error)
    }
}

/// A harvest that could not happen.
#[derive(Debug)]
pub enum ReadError {
    /// A file could not be read, or a sidecar inspected or removed.
    Io(io::Error),
    /// The database could not be opened, vacuumed, checkpointed or
    /// closed.
    Sqlite(sqlx::Error),
    /// The checkpoint was blocked: something still holds the
    /// database, and the gateway was supposed to be gone.
    Busy,
    /// The write-ahead log still carries this many bytes after the
    /// close: the fold did not happen, and the main file alone would
    /// lose committed transactions.
    WalRemains(u64),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Io(error) => {
                write!(f, "the continuation could not be read: {error}")
            }
            ReadError::Sqlite(error) => {
                write!(f, "state.db could not be folded: {error}")
            }
            ReadError::Busy => f.write_str(
                "the state.db checkpoint was blocked: something still holds the database",
            ),
            ReadError::WalRemains(len) => write!(
                f,
                "the state.db write-ahead log still holds {len} bytes after the fold"
            ),
        }
    }
}

impl error::Error for ReadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ReadError::Io(error) => Some(error),
            ReadError::Sqlite(error) => Some(error),
            ReadError::Busy | ReadError::WalRemains(_) => None,
        }
    }
}

impl From<io::Error> for ReadError {
    fn from(error: io::Error) -> Self {
        ReadError::Io(error)
    }
}

impl From<sqlx::Error> for ReadError {
    fn from(error: sqlx::Error) -> Self {
        ReadError::Sqlite(error)
    }
}
