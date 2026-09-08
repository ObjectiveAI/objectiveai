//! What the loop yields.

use diverge_provider_sdk::shared::containers::run_loop::response::AgenticLoopChunk;

use crate::continuation::Continuation;

/// One item of the loop's stream: a chunk of the conversation, or the
/// history at rest — whole, every call answered, nothing mid-turn —
/// for whoever keeps it to keep.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// One chunk of the loop's output.
    Chunk(AgenticLoopChunk),
    /// The history, at rest: yielded after every completed turn, and
    /// worth saving each time, because a container that dies keeps
    /// only what was saved.
    Rest(Continuation),
}
