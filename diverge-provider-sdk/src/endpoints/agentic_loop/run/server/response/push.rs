//! Coalescing a stream of chunks.

use super::AgenticLoopChunk;

/// Push one chunk onto `chunks`, coalescing the streamed kinds.
///
/// A stream delivers reasoning, text, refusals and tool-call
/// arguments in fragments; a record of the stream does not have to
/// keep the fragment boundaries. A chunk of one of those kinds,
/// arriving directly behind a chunk of the SAME kind — and, for a
/// tool call, of the same `id` — is pushed onto that chunk instead of
/// appended beside it, so consecutive fragments become one chunk, and
/// everything else appends as it came.
///
/// Order is preserved exactly: only adjacency merges, so a run broken
/// by any other kind stays broken, as it was on the wire.
pub fn push(chunks: &mut Vec<AgenticLoopChunk>, chunk: AgenticLoopChunk) {
    match (chunks.last_mut(), chunk) {
        (
            Some(AgenticLoopChunk::AssistantReasoning(last)),
            AgenticLoopChunk::AssistantReasoning(chunk),
        ) => last.push(chunk),
        (
            Some(AgenticLoopChunk::AssistantTextContent(last)),
            AgenticLoopChunk::AssistantTextContent(chunk),
        ) => last.push(chunk),
        (
            Some(AgenticLoopChunk::AssistantRefusal(last)),
            AgenticLoopChunk::AssistantRefusal(chunk),
        ) => last.push(chunk),
        (
            Some(AgenticLoopChunk::AssistantToolCall(last)),
            AgenticLoopChunk::AssistantToolCall(chunk),
        ) if last.id == chunk.id => last.push(chunk),
        (_, chunk) => chunks.push(chunk),
    }
}
