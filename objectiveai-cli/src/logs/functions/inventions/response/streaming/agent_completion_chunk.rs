//! Free-function port of
//! `functions::inventions::response::streaming::AgentCompletionChunk::produce_files`
//! (index wrapper around the agent-side completion chunk).

use objectiveai_sdk::functions::inventions::response::streaming::AgentCompletionChunk;

use objectiveai_sdk::logs::IndexedLogReference;

use crate::filesystem::logs::LogFile;

/// Produce log files for an agent completion within a function
/// invention. Returns `(reference, files)` where `reference` is an
/// [`IndexedLogReference`] carrying `index`. Files are written under
/// `agents/completions/`.
pub fn produce_files(
    c: &AgentCompletionChunk,
) -> (IndexedLogReference, Vec<LogFile>) {
    let (path, files) =
        match crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_files(&c.inner) {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
    (IndexedLogReference::new(path, c.index), files)
}
