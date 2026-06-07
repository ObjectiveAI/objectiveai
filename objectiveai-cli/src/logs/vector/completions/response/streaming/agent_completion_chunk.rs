//! Free-function ports of vector-tier
//! `AgentCompletionChunk::produce_files` (index wrapper around the
//! agent-side completion chunk) and `::produce_message_rows`.

use objectiveai_sdk::vector::completions::response::streaming::AgentCompletionChunk;

use objectiveai_sdk::logs::IndexedLogReference;

use crate::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

/// Produce log files for an agent completion within a vector
/// completion. Returns `(reference, files)` where `reference` is an
/// [`IndexedLogReference`] carrying `index`. Files are written under
/// `agents/completions/` (shared with standalone agent completions).
///
/// When the inner chunk's `id` is empty — the per-agent stream
/// errored before producing an ID-bearing chunk and therefore wrote
/// no log file — the reference stamps the inner chunk's `error`
/// onto itself via [`IndexedLogReference::with_error`] so the parent
/// vector completion log still surfaces what went wrong at that
/// swarm slot. When `id` is non-empty the error (if any) lives
/// inside the linked log file, so we don't duplicate it on the ref.
pub fn produce_files(
    c: &AgentCompletionChunk,
) -> (IndexedLogReference, Vec<LogFile>) {
    if c.inner.id.is_empty() {
        let reference = match &c.inner.error {
            Some(error) => IndexedLogReference::with_error(c.index, error.clone()),
            None => IndexedLogReference {
                r#type: objectiveai_sdk::logs::LogReferenceTag::Reference,
                path: None,
                index: c.index,
                error: None,
            },
        };
        return (reference, Vec::new());
    }
    let (inner_ref, files) =
        crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_files(&c.inner)
            .expect("inner produce_files returns Some when id is non-empty");
    (IndexedLogReference::new(inner_ref.path, c.index), files)
}

/// Delegates to the inner agent completion's message-row extractor.
pub fn produce_message_rows(
    c: &AgentCompletionChunk,
) -> impl Iterator<Item = MessageRow> + Send + '_ {
    crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_message_rows(&c.inner)
}
