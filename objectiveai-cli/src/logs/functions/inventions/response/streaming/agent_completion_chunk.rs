//! Free-function port of
//! `functions::inventions::response::streaming::AgentCompletionChunk::produce_files`
//! (index wrapper around the agent-side completion chunk).

use objectiveai_sdk::functions::inventions::response::streaming::AgentCompletionChunk;

use crate::filesystem::logs::{LogFile, indexed_reference};

/// Produce log files for an agent completion within a function
/// invention. Returns `(reference, files)` where `reference` is an
/// [`indexed_reference::LogReference`] carrying `index`. Files are
/// written under `agents/completions/`.
pub fn produce_files(
    c: &AgentCompletionChunk,
) -> (indexed_reference::LogReference, Vec<LogFile>) {
    let (path, files) =
        match crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_files(&c.inner) {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
    (
        indexed_reference::LogReference::new(path, c.index),
        files,
    )
}
