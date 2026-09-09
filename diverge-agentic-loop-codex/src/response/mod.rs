//! Everything `codex exec --json` writes on its stdout.
//!
//! The harness runs `codex exec --json` (and `codex exec resume
//! <thread_id> --json` after the first turn) and reads JSON Lines:
//! one event per line, `serde_json::to_string` of the exec crate's
//! `ThreadEvent`, and nothing else on stdout — the final message goes
//! to a file only with `-o`, and even a serialization failure is
//! written as an `error` event. These are those events, typed from
//! the source's own definitions: `codex-rs/exec/src/exec_events.rs`
//! at tag `rust-v0.153.4`, the version the image pins, with what the
//! JSONL event processor does with each recorded on the type.
//! Deserialize-only, but for the items the converter re-emits as a
//! notification's JSON (the todo list, a collab call, a file change,
//! a search action) and the usage the continuation keeps, which also
//! serialize.
//!
//! # Strict, deliberately
//!
//! A line that matches no known event fails to parse. The image pins
//! Codex's version, so the union is closed by construction, and a
//! reader that dies on a line the pinned version cannot write names a
//! bug rather than a circumstance. The one open tail is the source's
//! own: [`WebSearchAction::Other`](item::WebSearchAction::Other)
//! carries `#[serde(other)]` upstream. Unknown FIELDS pass, as they do
//! through the source's own serde.
//!
//! # The stream's shape
//!
//! - `thread.started` opens every process — a fresh thread and a
//!   resumed one alike — naming the thread the continuation resumes.
//! - One `turn.started`, then the items, then `turn.completed` (with
//!   the thread's CUMULATIVE usage, see [`Usage`]) or `turn.failed`;
//!   the process exits after that. One process is one turn. An
//!   INTERRUPTED turn ends with NEITHER: the processor writes nothing
//!   for it and shuts down, so stdout closing after `turn.started`
//!   with no terminal event is an interruption, not a bug.
//! - NO DELTAS. Agent messages and reasoning have no `item.started`:
//!   they arrive whole, once, at `item.completed`. Commands, MCP
//!   calls, web searches, collab calls and the todo list start and
//!   complete; a file change is documented by the source as
//!   completed only, though the mapping would pass a start through
//!   if the core sent one; only the todo list is `item.updated`, and
//!   it completes at the turn's end.
//! - The items are a PROJECTION of the core's, not all of it: a
//!   blank reasoning summary, four collab tools and an interrupted
//!   collab call produce no item; a declined patch is reported as
//!   failed. Each type says what its own projection folds.
//! - Non-fatal news — a warning, a config warning, a deprecation, a
//!   model reroute — is an `item.completed` whose item is an `error`.
//!   The top-level `error` event is a CRITICAL error that does not
//!   end the turn by itself; the turn then ends with `turn.failed`,
//!   whose error is the turn's own or the last critical one.

pub mod item;

mod error;
mod item_completed;
mod item_started;
mod item_updated;
mod thread_error;
mod thread_event;
mod thread_started;
mod turn_completed;
mod turn_failed;
mod turn_started;
mod usage;

pub use error::*;
pub use item_completed::*;
pub use item_started::*;
pub use item_updated::*;
pub use thread_error::*;
pub use thread_event::*;
pub use thread_started::*;
pub use turn_completed::*;
pub use turn_failed::*;
pub use turn_started::*;
pub use usage::*;
