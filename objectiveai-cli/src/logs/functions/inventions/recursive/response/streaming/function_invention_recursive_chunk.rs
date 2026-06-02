//! Free-function port of `FunctionInventionRecursiveChunk::produce_files`.

use objectiveai_sdk::functions::inventions::recursive::response::streaming::{
    FunctionInventionRecursiveChunk, FunctionInventionRecursiveChunkLog,
};

use objectiveai_sdk::logs::{IndexedLogReference, LogReference};

use crate::filesystem::logs::LogFile;

/// Produce the [`LogFile`]s for a recursive function invention chunk.
/// Returns `None` if the chunk has no ID yet. All paths relative to
/// `logs/`.
pub fn produce_files(
    c: &FunctionInventionRecursiveChunk,
) -> Option<(LogReference, Vec<LogFile>)> {
    const ROUTE: &str = "functions/inventions/recursive/response";

    let id = &c.id;
    if id.is_empty() {
        return None;
    }

    let mut files: Vec<LogFile> = Vec::new();
    let mut invention_refs: Vec<IndexedLogReference> = Vec::new();

    for invention in &c.inventions {
        let (reference, invention_files) =
            super::function_invention_chunk::produce_files(invention);
        invention_refs.push(reference);
        files.extend(invention_files);
    }

    let log = FunctionInventionRecursiveChunkLog {
        id: c.id.clone(),
        inventions: invention_refs,
        inventions_errors: c.inventions_errors,
        created: c.created,
        object: c.object,
        usage: c.usage.clone(),
    };

    let root_file = LogFile {
        route: ROUTE.to_string(),
        id: id.clone(),
        message_index: None,
        media_index: None,
        extension: "json".to_string(),
        content: serde_json::to_vec_pretty(&log).unwrap(),
    };
    let reference = LogReference::new(root_file.path());
    files.push(root_file);

    Some((reference, files))
}
