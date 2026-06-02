//! Free-function port of recursive `FunctionInventionChunk::produce_files`
//! (index wrapper around the non-recursive invention chunk).

use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionChunk;

use crate::filesystem::logs::{LogFile, indexed_reference};

/// Produce log files for an invention within a recursive invention.
/// Returns `(reference, files)` where `reference` is an
/// [`indexed_reference::LogReference`] carrying `index`. Files are
/// written under `functions/inventions/`.
pub fn produce_files(
    c: &FunctionInventionChunk,
) -> (indexed_reference::LogReference, Vec<LogFile>) {
    let (path, files) =
        match crate::logs::functions::inventions::response::streaming::function_invention_chunk::produce_files(&c.inner) {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
    (indexed_reference::LogReference::new(path, c.index), files)
}
