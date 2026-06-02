//! Free-function port of `VectorCompletionChunk::produce_files`.

use objectiveai_sdk::vector::completions::response::streaming::{
    VectorCompletionChunk, VectorCompletionChunkLog,
};

use objectiveai_sdk::logs::{IndexedLogReference, LogReference};

use crate::filesystem::logs::LogFile;

/// Produce the [`LogFile`]s for a vector completion chunk. Returns
/// `None` if the chunk has no ID yet. All paths relative to `logs/`.
/// Reference is a `{"type": "reference", "path": ...}` JSON value.
pub fn produce_files(
    c: &VectorCompletionChunk,
) -> Option<(LogReference, Vec<LogFile>)> {
    const ROUTE: &str = "vector/completions/response";

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

    let log = VectorCompletionChunkLog {
        id: c.id.clone(),
        completions: completion_refs,
        votes: c.votes.clone(),
        scores: c.scores.clone(),
        weights: c.weights.clone(),
        created: c.created,
        swarm: c.swarm.clone(),
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
