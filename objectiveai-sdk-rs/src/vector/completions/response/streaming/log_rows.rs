//! Streaming-content `log_rows` walker for vector-completion chunks.
//!
//! Recursively reaches into every embedded per-agent
//! [`AgentCompletionChunk`] and forwards its `log_rows()` output —
//! the writer dispatches every yielded row through the same path
//! regardless of which enclosing chunk produced it, since each agent
//! completion has its own globally-unique response id.

use crate::logs::{LogRowIter, LogValue};

use super::VectorCompletionChunk;

impl VectorCompletionChunk {
    pub fn log_rows<'a>(&'a self) -> LogRowIter<'a> {
        Box::new(
            self.completions
                .iter()
                .flat_map(|c| AgentCompletionChunkInVector(c).log_rows()),
        )
    }
}

/// Adapter that unwraps the vector-side wrapper around an agent
/// completion chunk and forwards to the underlying chunk's iterator.
struct AgentCompletionChunkInVector<'a>(&'a super::AgentCompletionChunk);

impl<'a> AgentCompletionChunkInVector<'a> {
    fn log_rows(&self) -> Box<dyn Iterator<Item = LogValue<'a>> + Send + 'a> {
        // The vector-side AgentCompletionChunk type is the wire wrapper
        // around the bare agent-completion chunk. The flattened
        // `inner` is the agent-completion chunk proper.
        self.0.inner.log_rows()
    }
}
