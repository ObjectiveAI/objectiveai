//! What the loop yields.

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;

/// One item of the loop's stream: a chunk of the conversation, or —
/// last, when the loop is done or has died with progress worth
/// keeping — the continuation's bytes, the closer.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// One chunk of the loop's output.
    Chunk(AgenticLoopChunk),
    /// The continuation, whole: the history the next run resumes
    /// from. Yielded once, and nothing follows it.
    Continuation(Vec<u8>),
}
