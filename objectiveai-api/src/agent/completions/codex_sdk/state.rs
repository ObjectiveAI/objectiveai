/// Per-turn upstream state for the Codex SDK client. Surfaced as the
/// trailing [`super::super::StreamItem::State`] of each agent-completion
/// stream and persisted into the public [`Continuation`] so subsequent
/// requests can `--resume` the same Codex thread.
///
/// [`Continuation`]: objectiveai::agent::codex_sdk::Continuation
#[derive(Debug, Clone)]
pub struct State {
    /// The Codex thread id surfaced by the runner. Empty string if no
    /// `thread.started` event was observed.
    pub thread_id: String,
    /// Number of assistant messages produced in this turn. Codex is a
    /// single-turn-per-request runner so this is almost always 1; we
    /// track it for parity with the claude pattern (and to allow
    /// multi-`agent_message` turns if codex ever emits them).
    pub message_count: u64,
}
