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
//! that file; a file longer than a piece is several chunks with the
//! same tag. No lengths, no envelope: the boundaries the protocol
//! preserves are the framing.
//!
//! # The flow, and what it spares
//!
//! The container is fresh. The continuation lands ([`Ingest`])
//! before `hermes gateway` ever starts, so there is nothing stale to
//! clear and nothing else touching the files; the gateway runs; it
//! exits; the harvest ([`stream`]) folds the database and reads the
//! files. Nothing is validated that the flow already guarantees: a
//! chunk goes to the file its tag names, in whatever order chunks
//! come, and the one thing judged before the gateway starts is that
//! a delivered `state.db` opens ([`check`]).
//!
//! # Never whole in memory
//!
//! A database grows for the life of a lineage and memories grow
//! with it, and both will be large. So nothing here holds a
//! continuation: on the way IN, [`Ingest`] appends each chunk to
//! its file the moment it lands and keeps only the open handle; on
//! the way OUT, [`stream`] reads each file [`PIECE`] bytes at a
//! time and yields each piece as it is read, one alive at once.
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

mod check;
mod check_error;
mod db;
mod ingest;
mod ingest_error;
mod read_error;
mod stream;

pub use check::*;
pub use check_error::*;
pub use ingest::*;
pub use ingest_error::*;
pub use read_error::*;
pub use stream::*;

use std::path::{Path, PathBuf};

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

/// The three, in the order the harvest sends them.
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
