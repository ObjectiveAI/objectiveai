//! What a Hermes continuation holds — the session store and the two
//! memory files — and how it moves: to disk as it arrives, from disk
//! as it leaves, never whole in memory.
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
//! that file. A file longer than a piece spans several consecutive
//! chunks with the same tag; a present-but-empty file is one
//! tag-only chunk; the files come in ascending tag order. No
//! lengths, no envelope: the boundaries the protocol preserves are
//! the framing.
//!
//! # Never whole in memory
//!
//! A database grows for the life of a lineage and memories grow
//! with it, and both will be large. So nothing here holds a
//! continuation: on the way IN, [`Ingest`] appends each chunk to
//! its file the moment it lands and keeps only the open handle; on
//! the way OUT, [`stream`] reads each file [`PIECE`] bytes at a
//! time and yields each piece as it is read, one alive at once. The
//! ingest's order rule — files ascending, each contiguous — is
//! exactly the order the stream sends them in.
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

use futures_util::Stream;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
use sqlx::{ConnectOptions as _, Connection as _, Executor as _, Row as _};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

/// Where Hermes keeps its state, fixed for the container's life: the
/// default home for the container's root user, with no
/// `HERMES_HOME` override — so what [`Ingest`] lays down is what the
/// gateway finds, and what the gateway leaves is what [`stream`]
/// sweeps up.
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

/// The three, in the order they are sent and must arrive.
const TAGS: [u8; 3] = [STATE_DB_TAG, MEMORY_TAG, USER_TAG];

/// How much of a file one outbound chunk carries — the most of the
/// continuation that is ever in memory at once. Two mebibytes, well
/// under the protocol's four-mebibyte chunk ceiling.
pub const PIECE: usize = 2 * 1024 * 1024;

/// The file a tag names, under the home — or none, for a tag that
/// is not one of the three.
fn path_for(tag: u8) -> Option<PathBuf> {
    let home = Path::new(HERMES_HOME);
    match tag {
        STATE_DB_TAG => Some(home.join(STATE_DB)),
        MEMORY_TAG => Some(home.join(MEMORIES).join(MEMORY_FILE)),
        USER_TAG => Some(home.join(MEMORIES).join(USER_FILE)),
        _ => None,
    }
}

/// Which of the three files a delivery put on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landed {
    /// `state.db` — every real continuation has one; `false` here is
    /// the fresh start.
    pub state_db: bool,
    /// `memories/MEMORY.md`.
    pub memory: bool,
    /// `memories/USER.md`.
    pub user: bool,
}

impl Landed {
    /// Nothing landed: the fresh start.
    pub const NONE: Landed = Landed {
        state_db: false,
        memory: false,
        user: false,
    };

    fn mark(&mut self, tag: u8) {
        match tag {
            STATE_DB_TAG => self.state_db = true,
            MEMORY_TAG => self.memory = true,
            USER_TAG => self.user = true,
            _ => {}
        }
    }
}

/// A continuation being written to disk as it arrives.
///
/// Started on the first chunk, fed one chunk at a time, finished
/// once. Each chunk's payload is appended to the file its tag names
/// the moment it lands; between chunks nothing is held but the open
/// handle of the file currently being written. The rules a delivery
/// must keep, each with its refusal: a chunk with no bytes at all
/// has no tag ([`IngestError::Empty`]); a tag that is none of the
/// three ([`UnknownTag`](IngestError::UnknownTag)); a file's chunks
/// must be contiguous and the files in ascending tag order, so a tag
/// lower than the last seen is a sequence reordered or interleaved
/// ([`Order`](IngestError::Order)); and a continuation with no
/// `state.db` is no continuation
/// ([`MissingStateDb`](IngestError::MissingStateDb), judged at the
/// finish).
pub struct Ingest {
    /// The tag before this one, for the order rule.
    last: Option<u8>,
    /// The file the current tag appends to, and which tag it is.
    open: Option<(u8, File)>,
    /// Which files exist so far.
    landed: Landed,
}

impl Ingest {
    /// Make room: the memory directory created, and any stale
    /// `state.db` family removed — a leftover write-ahead log beside
    /// a fresh copy would be replayed INTO it. Called by the store on
    /// the first chunk, so a fresh start (no chunks) touches nothing.
    pub async fn start() -> io::Result<Self> {
        let home = Path::new(HERMES_HOME);
        tokio::fs::create_dir_all(home.join(MEMORIES)).await?;
        let db = home.join(STATE_DB);
        remove_if_present(&db).await?;
        for suffix in ["-wal", "-shm", "-journal"] {
            remove_if_present(&sidecar(&db, suffix)).await?;
        }
        Ok(Ingest {
            last: None,
            open: None,
            landed: Landed::NONE,
        })
    }

    /// Land one chunk: judge its tag, append its payload to the
    /// tag's file. A new tag closes the file before it and opens
    /// its own — created even when this chunk is tag-only, so an
    /// empty file round-trips as present.
    pub async fn push(&mut self, chunk: &[u8]) -> Result<(), IngestError> {
        let (tag, payload) = chunk.split_first().ok_or(IngestError::Empty)?;
        let path = path_for(*tag).ok_or(IngestError::UnknownTag(*tag))?;
        if let Some(after) = self.last
            && *tag < after
        {
            return Err(IngestError::Order { tag: *tag, after });
        }
        self.last = Some(*tag);

        if self.open.as_ref().is_none_or(|(open, _)| *open != *tag) {
            self.close_open().await?;
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await?;
            self.open = Some((*tag, file));
            self.landed.mark(*tag);
        }
        let (_, file) = self.open.as_mut().expect("opened just above");
        file.write_all(payload).await?;
        Ok(())
    }

    /// The delivery is complete: flush and close the last file, and
    /// say what landed. A delivery that never carried `state.db`
    /// fails here — there is nothing to resume from.
    pub async fn finish(mut self) -> Result<Landed, IngestError> {
        self.close_open().await?;
        if !self.landed.state_db {
            return Err(IngestError::MissingStateDb);
        }
        Ok(self.landed)
    }

    /// Flush and sync the open file, if any, and let it go.
    async fn close_open(&mut self) -> io::Result<()> {
        if let Some((_, mut file)) = self.open.take() {
            file.flush().await?;
            file.sync_all().await?;
        }
        Ok(())
    }
}

/// Prove the delivered database opens: `PRAGMA quick_check`, one
/// sequential read of the file, once per run — before the gateway
/// starts.
///
/// Not caution for its own sake: Hermes HEALS a database it cannot
/// open, by quarantining it and starting fresh, and a run that
/// started fresh would harvest an amnesiac continuation over the
/// lineage without anyone noticing. So a delivery that does not
/// check out fails loudly here instead ([`CheckError::Corrupt`]).
pub async fn check() -> Result<(), CheckError> {
    let db = Path::new(HERMES_HOME).join(STATE_DB);
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
        return Err(CheckError::Corrupt(verdict));
    }
    Ok(())
}

/// Harvest the state under [`HERMES_HOME`] as the chunks the run
/// closes with — [`Ingest`]'s exact inverse, a piece at a time.
/// Called after the gateway process has exited: nothing else may
/// hold the database.
///
/// The database is folded before anything is read: `VACUUM` first,
/// which rewrites every page (and reads every page, so damage
/// surfaces here) THROUGH the write-ahead log, then a
/// `wal_checkpoint(TRUNCATE)`, which moves the log's frames into
/// the main file and empties the log; then the connection is
/// closed, which — being the last — deletes the log and the
/// shared-memory file. A checkpoint that could not complete means
/// something still holds the database ([`ReadError::Busy`]); a log
/// still carrying bytes after the close means the fold did not
/// happen ([`ReadError::WalRemains`]). `VACUUM` needs up to twice
/// the file's size free under the home while it runs. That much is
/// done before this returns, so a fold that fails is an `Err` here
/// and never a stream that dies mid-way.
///
/// Then the stream: the three files in tag order, `state.db`
/// required and the memory files skipped when absent, each read
/// [`PIECE`] bytes at a time and yielded behind its tag as soon as
/// that piece is read — one piece alive at once. An empty file is
/// one tag-only chunk. The consumer sends each item as one
/// continuation frame and drops it.
pub async fn stream()
-> Result<impl Stream<Item = Result<Vec<u8>, ReadError>>, ReadError> {
    let db = Path::new(HERMES_HOME).join(STATE_DB);
    fold(&db).await?;

    Ok(async_stream::try_stream! {
        for tag in TAGS {
            let path = path_for(tag).expect("one of the three");
            let mut file = match File::open(&path).await {
                Ok(file) => file,
                Err(error)
                    if tag != STATE_DB_TAG
                        && error.kind() == io::ErrorKind::NotFound =>
                {
                    continue;
                }
                Err(error) => Err(ReadError::Io(error))?,
            };
            let mut first = true;
            loop {
                // The tag, then up to PIECE bytes of the file: read
                // until the piece is full or the file is done.
                let mut piece = Vec::with_capacity(PIECE + 1);
                piece.push(tag);
                while piece.len() < PIECE + 1 {
                    if file.read_buf(&mut piece).await? == 0 {
                        break;
                    }
                }
                let done = piece.len() < PIECE + 1;
                // A tag-only piece is the empty file's one chunk —
                // but after a full piece it is just the end.
                if piece.len() > 1 || first {
                    yield piece;
                }
                first = false;
                if done {
                    break;
                }
            }
        }
    })
}

/// Fold the write-ahead log into `state.db` and leave a lone file:
/// see [`stream`].
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

/// A delivery that could not be landed.
#[derive(Debug)]
pub enum IngestError {
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
    /// The delivery finished without a `state.db` chunk.
    MissingStateDb,
    /// A file could not be prepared, appended to, or closed.
    Io(io::Error),
}

impl fmt::Display for IngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngestError::Empty => {
                f.write_str("a continuation chunk is empty")
            }
            IngestError::UnknownTag(tag) => {
                write!(f, "unknown continuation chunk tag {tag}")
            }
            IngestError::Order { tag, after } => write!(
                f,
                "continuation chunk tag {tag} arrived after tag {after}"
            ),
            IngestError::MissingStateDb => {
                f.write_str("the continuation carries no state.db")
            }
            IngestError::Io(error) => {
                write!(f, "the continuation could not be written: {error}")
            }
        }
    }
}

impl error::Error for IngestError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            IngestError::Io(error) => Some(error),
            IngestError::Empty
            | IngestError::UnknownTag(_)
            | IngestError::Order { .. }
            | IngestError::MissingStateDb => None,
        }
    }
}

impl From<io::Error> for IngestError {
    fn from(error: io::Error) -> Self {
        IngestError::Io(error)
    }
}

/// A delivered database that did not check out.
#[derive(Debug)]
pub enum CheckError {
    /// The database could not be opened or asked.
    Sqlite(sqlx::Error),
    /// The database opened but did not check out: `quick_check`'s
    /// findings, verbatim.
    Corrupt(Vec<String>),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::Sqlite(error) => {
                write!(f, "the delivered state.db could not be opened: {error}")
            }
            CheckError::Corrupt(findings) => write!(
                f,
                "the delivered state.db is corrupt: {}",
                findings.join("; ")
            ),
        }
    }
}

impl error::Error for CheckError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            CheckError::Sqlite(error) => Some(error),
            CheckError::Corrupt(_) => None,
        }
    }
}

impl From<sqlx::Error> for CheckError {
    fn from(error: sqlx::Error) -> Self {
        CheckError::Sqlite(error)
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
