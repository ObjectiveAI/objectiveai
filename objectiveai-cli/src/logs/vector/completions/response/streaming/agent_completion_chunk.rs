//! Free-function ports of vector-tier
//! `AgentCompletionChunk::produce_files` (index wrapper around the
//! agent-side completion chunk) and `::produce_message_rows`.

use objectiveai_sdk::vector::completions::response::streaming::AgentCompletionChunk;

use objectiveai_sdk::logs::IndexedLogReference;

use crate::filesystem::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

/// Produce log files for an agent completion within a vector
/// completion. Returns `(reference, files)` where `reference` is an
/// [`IndexedLogReference`] carrying `index`. Files are written under
/// `agents/completions/` (shared with standalone agent completions).
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

/// Delegates to the inner agent completion's message-row extractor.
pub fn produce_message_rows(
    c: &AgentCompletionChunk,
) -> impl Iterator<Item = MessageRow> + Send + '_ {
    crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_message_rows(&c.inner)
}
