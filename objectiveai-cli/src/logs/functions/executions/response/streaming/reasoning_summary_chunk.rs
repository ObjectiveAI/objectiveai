//! Free-function ports of `ReasoningSummaryChunk::produce_files` and
//! `ReasoningSummaryChunk::produce_message_rows`.

use objectiveai_sdk::functions::executions::response::streaming::{
    ReasoningSummaryChunk, reasoning_summary_log_reference,
};

use crate::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

/// Produce log files for a reasoning summary. Returns
/// `(reference, files)` where `reference` carries the wrapper's own
/// optional `error`. Files under `agents/completions/`.
pub fn produce_files(
    c: &ReasoningSummaryChunk,
) -> (reasoning_summary_log_reference::LogReference, Vec<LogFile>) {
    let (path, files) =
        match crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_files(&c.inner) {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
    let mut reference =
        reasoning_summary_log_reference::LogReference::new(path);
    if let Some(error) = &c.error {
        reference.error = Some(serde_json::to_value(error).unwrap());
    }
    (reference, files)
}

/// Delegates to the inner agent completion's message-row extractor.
pub fn produce_message_rows(
    c: &ReasoningSummaryChunk,
) -> impl Iterator<Item = MessageRow> + Send + '_ {
    crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_message_rows(&c.inner)
}
