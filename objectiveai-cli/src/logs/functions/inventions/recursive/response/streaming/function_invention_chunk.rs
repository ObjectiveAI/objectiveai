//! Free-function ports of recursive `FunctionInventionChunk::produce_files`
//! (index wrapper around the non-recursive invention chunk) and
//! `::produce_message_rows`.

use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionChunk;

use objectiveai_sdk::logs::IndexedLogReference;

use crate::filesystem::db::schema::MessageRow;
use crate::filesystem::logs::LogFile;

/// Produce log files for an invention within a recursive invention.
/// Returns `(reference, files)` where `reference` is an
/// [`IndexedLogReference`] carrying `index`. Files are written under
/// `functions/inventions/`.
pub fn produce_files(
    c: &FunctionInventionChunk,
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
        crate::logs::functions::inventions::response::streaming::function_invention_chunk::produce_files(&c.inner)
            .expect("inner produce_files returns Some when id is non-empty");
    (IndexedLogReference::new(inner_ref.path, c.index), files)
}

/// Delegates to the inner non-recursive invention.
pub fn produce_message_rows(
    c: &FunctionInventionChunk,
) -> impl Iterator<Item = MessageRow> + Send + '_ {
    crate::logs::functions::inventions::response::streaming::function_invention_chunk::produce_message_rows(&c.inner)
}
