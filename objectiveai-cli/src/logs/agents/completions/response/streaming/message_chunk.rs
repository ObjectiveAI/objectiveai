//! Free-function port of `MessageChunk::produce_files` (dispatch).

use objectiveai_sdk::agent::completions::response::streaming::MessageChunk;

use crate::filesystem::logs::{LogFile, LogReference};

/// Produce log files for a `MessageChunk`. Returns `(reference, files)`
/// where `reference` points to this message's file and `files`
/// contains all produced files.
pub fn produce_files(
    c: &MessageChunk,
    id: &str,
    route_base: &str,
) -> (LogReference, Vec<LogFile>) {
    match c {
        MessageChunk::Assistant(chunk) => super::assistant_response_chunk::produce_files(
            chunk, id, route_base,
        ),
        MessageChunk::Tool(chunk) => super::super::tool_response::produce_files(
            chunk, id, route_base,
        ),
    }
}
