//! Free-function ports of
//! `functions::inventions::response::streaming::AgentCompletionChunk::produce_files`
//! (index wrapper around the agent-side completion chunk) and
//! `::produce_message_rows`.

use objectiveai_sdk::functions::inventions::response::streaming::AgentCompletionChunk;

use objectiveai_sdk::logs::IndexedLogReference;

use crate::filesystem::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

/// Produce log files for an agent completion within a function
/// invention. Returns `(reference, files)` where `reference` is an
/// [`IndexedLogReference`] carrying `index`. Files are written under
/// `agents/completions/`.
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
