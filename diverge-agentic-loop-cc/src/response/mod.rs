//! Everything Claude Code writes on its stream-json stdout.
//!
//! The harness runs `claude -p --output-format stream-json --verbose`
//! and reads NDJSON: one JSON record per line, guaranteed parseable —
//! Claude Code installs a stdout guard that diverts anything else to
//! stderr. These are those records, typed from the source's own
//! schemas: the full `StdoutMessage` union, including the members
//! only other transports or flags ever send, because a reader of the
//! union reads all of it.
//!
//! # Strict, deliberately
//!
//! A line that matches no known record fails to parse. The image
//! pins Claude Code's version, so the union is closed by
//! construction — and the source's own schema-escaping emissions
//! (`system/bridge_state`, `task_progress.workflow_progress`, the
//! `auto` permission mode) are typed as KNOWN, so strictness rejects
//! only what the pinned version cannot say. Unknown FIELDS pass, as
//! they do through the source's own non-strict schemas.
//!
//! # Reading order matters less than it looks
//!
//! `system/init` opens every TURN, not every process; assistant and
//! user messages arrive one content block per record; a `result` can
//! be held back past later records while background agents drain;
//! and the authoritative turn-over signal is
//! `system/session_state_changed { state: idle }`. Each type's docs
//! carry the details.

pub mod message;

mod assistant;
mod auth_status;
mod control;
mod keep_alive;
mod prompt_suggestion;
mod rate_limit_event;
mod result;
mod stream_event;
mod streamlined;
mod system;
mod tool_progress;
mod tool_use_summary;
mod user;

pub use assistant::*;
pub use auth_status::*;
pub use control::*;
pub use keep_alive::*;
pub use prompt_suggestion::*;
pub use rate_limit_event::*;
pub use result::*;
pub use stream_event::*;
pub use streamlined::*;
pub use system::*;
pub use tool_progress::*;
pub use tool_use_summary::*;
pub use user::*;

use serde::{Deserialize, Serialize};

/// One line of stdout, whichever record it is — the source's
/// `StdoutMessage` union, whole, discriminated by `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StdoutMessage {
    /// The run narrating itself; see [`System`] for the subtypes.
    System(System),
    /// One block of what the model said.
    Assistant(Assistant),
    /// One block of what went back to it.
    User(User),
    /// How the turn ended; see [`Result`] for the subtypes.
    Result(Result),
    /// A raw API streaming event, with `--include-partial-messages`.
    StreamEvent(StreamEvent),
    /// A long tool run's heartbeat, on remote builds.
    ToolProgress(ToolProgress),
    /// Tool calls, summarized after the fact.
    ToolUseSummary(ToolUseSummary),
    /// Subscription rate limits moving.
    RateLimitEvent(RateLimitEvent),
    /// Authentication narrated, with `--enable-auth-status`.
    AuthStatus(AuthStatus),
    /// A suggested follow-up, when suggestions are on.
    PromptSuggestion(PromptSuggestion),
    /// An ask travelling outward — a permission or sandbox prompt.
    ControlRequest(ControlRequest),
    /// The answer to an ask that arrived on stdin.
    ControlResponse(ControlResponse),
    /// An open ask, withdrawn.
    ControlCancelRequest(ControlCancelRequest),
    /// A transport heartbeat carrying nothing.
    KeepAlive(KeepAlive),
    /// The internal terse mode's text record.
    StreamlinedText(StreamlinedText),
    /// The internal terse mode's tool summary record.
    StreamlinedToolUseSummary(StreamlinedToolUseSummary),
}
