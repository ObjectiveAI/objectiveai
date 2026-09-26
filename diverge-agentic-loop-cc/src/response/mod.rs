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

pub mod assistant;
pub mod auth_status;
pub mod control;
pub mod keep_alive;
pub mod message;
pub mod prompt_suggestion;
pub mod rate_limit_event;
pub mod result;
pub mod stream_event;
pub mod streamlined;
pub mod system;
pub mod tool_progress;
pub mod tool_use_summary;
pub mod user;

use diverge_sdk::provider::endpoints::containers::agents::run::server::response;
use serde::Deserialize;

/// One line of stdout, whichever record it is — the source's
/// `StdoutMessage` union, whole.
///
/// Untagged, the way this crate models unions: every record carries
/// its own `type` literal as a marker field (and its `subtype` where
/// it has one), so a record is self-describing wherever it travels —
/// nested inside another record just as at the top of a line — and
/// only the right variant can accept a given literal. What a tagged
/// parent would have hoisted out of the child stays on the child,
/// which is where the source's schemas put it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum StdoutMessage {
    /// The run narrating itself; see [`system::System`] for the subtypes.
    System(system::System),
    /// One block of what the model said.
    Assistant(assistant::Assistant),
    /// One block of what went back to it.
    User(user::User),
    /// How the turn ended; see [`result::Result`] for the subtypes.
    Result(result::Result),
    /// A raw API streaming event, with `--include-partial-messages`.
    StreamEvent(stream_event::StreamEvent),
    /// A long tool run's heartbeat, on remote builds.
    ToolProgress(tool_progress::ToolProgress),
    /// Tool calls, summarized after the fact.
    ToolUseSummary(tool_use_summary::ToolUseSummary),
    /// Subscription rate limits moving.
    RateLimitEvent(rate_limit_event::RateLimitEvent),
    /// Authentication narrated, with `--enable-auth-status`.
    AuthStatus(auth_status::AuthStatus),
    /// A suggested follow-up, when suggestions are on.
    PromptSuggestion(prompt_suggestion::PromptSuggestion),
    /// An ask travelling outward — a permission or sandbox prompt.
    ControlRequest(control::ControlRequest),
    /// The answer to an ask that arrived on stdin.
    ControlResponse(control::ControlResponse),
    /// An open ask, withdrawn.
    ControlCancelRequest(control::ControlCancelRequest),
    /// A transport heartbeat carrying nothing.
    KeepAlive(keep_alive::KeepAlive),
    /// The internal terse mode's text record.
    StreamlinedText(streamlined::StreamlinedText),
    /// The internal terse mode's tool summary record.
    StreamlinedToolUseSummary(streamlined::StreamlinedToolUseSummary),
}

impl StdoutMessage {
    /// This record, as the chunks it means — pure dispatch, the
    /// silent kinds enumerated.
    ///
    /// Only three records speak chunks: the assistant's content, the
    /// user's tool results, and the result's bill. Everything else
    /// is narration, transport, or machinery the harness already
    /// consumed — `system` (all subtypes), stream events (never
    /// requested: no `--include-partial-messages`), tool progress
    /// and summaries, rate-limit and auth narration, prompt
    /// suggestions, the control records (the reader taps
    /// `control_response` before conversion runs), keepalives, and
    /// the terse mode's records. Replays are the reader's too: its
    /// tap resolves and marks them first, and
    /// [`user::User::into_chunks`] skips them.
    pub fn into_chunks(
        self,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        match self {
            StdoutMessage::Assistant(record) => {
                record.into_chunks(chunks);
            }
            StdoutMessage::User(record) => record.into_chunks(chunks),
            StdoutMessage::Result(record) => record.into_chunks(chunks),
            StdoutMessage::System(_)
            | StdoutMessage::StreamEvent(_)
            | StdoutMessage::ToolProgress(_)
            | StdoutMessage::ToolUseSummary(_)
            | StdoutMessage::RateLimitEvent(_)
            | StdoutMessage::AuthStatus(_)
            | StdoutMessage::PromptSuggestion(_)
            | StdoutMessage::ControlRequest(_)
            | StdoutMessage::ControlResponse(_)
            | StdoutMessage::ControlCancelRequest(_)
            | StdoutMessage::KeepAlive(_)
            | StdoutMessage::StreamlinedText(_)
            | StdoutMessage::StreamlinedToolUseSummary(_) => {}
        }
    }
}
