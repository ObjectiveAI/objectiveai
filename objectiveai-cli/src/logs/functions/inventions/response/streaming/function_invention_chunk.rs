//! Free-function port of `FunctionInventionChunk::produce_files`.

use objectiveai_sdk::functions::inventions::response::streaming::{
    FunctionInventionChunk, FunctionInventionChunkLog,
};

use objectiveai_sdk::logs::{IndexedLogReference, LogReference};

use crate::filesystem::logs::LogFile;

/// Produce the [`LogFile`]s for a function invention chunk. Returns
/// `None` if the chunk has no ID yet. All paths relative to `logs/`.
pub fn produce_files(
    c: &FunctionInventionChunk,
) -> Option<(LogReference, Vec<LogFile>)> {
    const ROUTE: &str = "functions/inventions/response";

    let id = &c.id;
    if id.is_empty() {
        return None;
    }

    let mut files: Vec<LogFile> = Vec::new();
    let mut completion_refs: Vec<IndexedLogReference> = Vec::new();

    for completion in &c.completions {
        let (reference, completion_files) =
            super::agent_completion_chunk::produce_files(completion);
        completion_refs.push(reference);
        files.extend(completion_files);
    }

    let log = FunctionInventionChunkLog {
        id: c.id.clone(),
        completions: completion_refs,
        state: c.state.clone(),
        path: c.path.clone(),
        function: c.function.clone(),
        created: c.created,
        object: c.object,
        usage: c.usage.clone(),
        error: c.error.clone(),
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
