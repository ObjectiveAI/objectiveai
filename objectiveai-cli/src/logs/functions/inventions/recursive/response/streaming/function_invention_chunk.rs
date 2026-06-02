//! Free-function port of recursive `FunctionInventionChunk::produce_files`
//! (index wrapper around the non-recursive invention chunk).

use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionChunk;

use objectiveai_sdk::logs::IndexedLogReference;

use crate::filesystem::logs::LogFile;

/// Produce log files for an invention within a recursive invention.
/// Returns `(reference, files)` where `reference` is an
/// [`IndexedLogReference`] carrying `index`. Files are written under
/// `functions/inventions/`.
pub fn produce_files(
    c: &FunctionInventionChunk,
) -> (IndexedLogReference, Vec<LogFile>) {
    let (path, files) =
        match crate::logs::functions::inventions::response::streaming::function_invention_chunk::produce_files(&c.inner) {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
    (IndexedLogReference::new(path, c.index), files)
}
