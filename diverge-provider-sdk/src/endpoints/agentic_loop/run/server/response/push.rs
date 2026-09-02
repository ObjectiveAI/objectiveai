//! Coalescing a stream of chunks.

use super::{AgenticLoopChunk, AssistantToolCallChunk};

/// Push one chunk onto `chunks`, coalescing the streamed kinds.
///
/// A stream delivers reasoning, text, refusals and tool-call
/// arguments in fragments; a record of the stream does not have to
/// keep the fragment boundaries. The text kinds merge by ADJACENCY: a
/// chunk arriving directly behind one of the same kind is pushed onto
/// it, so consecutive fragments become one chunk and a run broken by
/// any other kind stays broken, as it was on the wire.
///
/// Tool calls merge by ID, searched backwards, within the most recent
/// chain — everything after the last tool response, or everything, if
/// there is none. A model streaming several calls at once interleaves
/// their fragments, so a fragment's home may not be the last element;
/// but a call never continues past a tool response, so the search
/// stops there, and an id seen again beyond that boundary is a new
/// call.
///
/// Nothing merges across threads: every merge also requires equal
/// `parent_tool_call_id`, so a sub-agent's fragment never fuses onto
/// the main thread's chunk (or another sub-agent's), whatever the
/// adjacency. The boundary between threads is as real as any other
/// kind's.
pub fn push(chunks: &mut Vec<AgenticLoopChunk>, chunk: AgenticLoopChunk) {
    match (chunks.last_mut(), chunk) {
        (
            Some(AgenticLoopChunk::AssistantReasoning(last)),
            AgenticLoopChunk::AssistantReasoning(chunk),
        ) if last.parent_tool_call_id == chunk.parent_tool_call_id => {
            last.push(chunk)
        }
        (
            Some(AgenticLoopChunk::AssistantTextContent(last)),
            AgenticLoopChunk::AssistantTextContent(chunk),
        ) if last.parent_tool_call_id == chunk.parent_tool_call_id => {
            last.push(chunk)
        }
        (
            Some(AgenticLoopChunk::AssistantRefusal(last)),
            AgenticLoopChunk::AssistantRefusal(chunk),
        ) if last.parent_tool_call_id == chunk.parent_tool_call_id => {
            last.push(chunk)
        }
        (_, AgenticLoopChunk::AssistantToolCall(chunk)) => {
            push_tool_call(chunks, chunk)
        }
        (_, chunk) => chunks.push(chunk),
    }
}

/// The tool-call merge: backwards through the chain since the last
/// tool response, onto the most recent fragment of the same call on
/// the same thread.
fn push_tool_call(
    chunks: &mut Vec<AgenticLoopChunk>,
    chunk: AssistantToolCallChunk,
) {
    for existing in chunks.iter_mut().rev() {
        match existing {
            AgenticLoopChunk::ToolResponse(_) => break,
            AgenticLoopChunk::AssistantToolCall(existing)
                if existing.id == chunk.id
                    && existing.parent_tool_call_id
                        == chunk.parent_tool_call_id =>
            {
                existing.push(chunk);
                return;
            }
            _ => {}
        }
    }
    chunks.push(AgenticLoopChunk::AssistantToolCall(chunk));
}
