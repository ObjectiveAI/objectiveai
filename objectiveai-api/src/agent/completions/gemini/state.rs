/// Per-turn upstream state for the Gemini client. Surfaced as the
/// trailing [`super::super::StreamItem::State`] of each agent-completion
/// stream.
///
/// The gemini runner is stateless — there is no server-side session to
/// resume. Continuation works by replaying the full conversation, so the
/// state tracks the assistant `message_count` produced this turn (for
/// the orchestrator's continuation index bookkeeping, mirroring the
/// codex pattern). `session_id` is a stable UUID minted on the first
/// turn purely so the public continuation carries a consistent id.
#[derive(Debug, Clone)]
pub struct State {
    /// Stable session id minted on the first turn. Carried purely so
    /// the public [`Continuation`] has a consistent `session_id`; it is
    /// NOT used to resume any server session (the runner is stateless).
    ///
    /// [`Continuation`]: objectiveai_sdk::agent::gemini::Continuation
    pub session_id: String,
    /// Number of assistant messages produced in this turn.
    pub message_count: u64,
    /// The FULL conversation this turn (prior history + this turn's
    /// input messages) PLUS the model turn the runner produced
    /// (assistant text + tool calls) and any tool results it resolved
    /// internally, all in the canonical agent-completions message shape.
    /// Becomes the new public continuation history so the next stateless
    /// replay carries everything — including the model's own prior
    /// answer, which the runner won't remember on its own. The API
    /// translates these canonical messages back into the runner's wire
    /// shape at request time.
    pub messages: Vec<objectiveai_sdk::agent::completions::message::Message>,
}
