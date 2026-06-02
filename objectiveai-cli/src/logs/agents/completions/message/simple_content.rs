//! Free-function port of `SimpleContent::extract_media`.

use objectiveai_sdk::agent::completions::message::{
    SimpleContent, SimpleContentLog, SimpleContentPart,
};

use crate::filesystem::logs::{LogFile, LogReference};

/// Extract every chunk of a `SimpleContent` into its own on-disk log
/// file, returning a [`SimpleContentLog`] of [`LogReference`]s pointing
/// at the written files.
///
/// `media_root` is the parent directory under which the `text` subdir
/// gets created (SimpleContent has only text parts).
///
/// - `SimpleContent::Text(text)` → one `.txt` at
///   `<media_root>/text/<id>-<idx>.txt` containing `text.into_bytes()`.
///   Returns `Reference(ref)`.
/// - `SimpleContent::Parts(parts)` → one `.txt` per part at
///   `<media_root>/text/<id>-<idx>-<part_idx>.txt`. Returns
///   `Parts(vec_of_refs)`.
pub fn extract_media(
    content: SimpleContent,
    media_root: &str,
    id: &str,
    message_index: u64,
) -> (SimpleContentLog, Vec<LogFile>) {
    match content {
        SimpleContent::Text(text) => {
            let log_file = LogFile {
                route: format!("{media_root}/text"),
                id: id.to_string(),
                message_index: Some(message_index),
                media_index: None,
                extension: "txt".to_string(),
                content: text.into_bytes(),
            };
            let reference = LogReference::new(log_file.path());
            (SimpleContentLog::Reference(reference), vec![log_file])
        }
        SimpleContent::Parts(parts) => {
            let mut log_refs = Vec::with_capacity(parts.len());
            let mut files = Vec::with_capacity(parts.len());
            for (part_idx, part) in parts.into_iter().enumerate() {
                let SimpleContentPart::Text { text } = part;
                let log_file = LogFile {
                    route: format!("{media_root}/text"),
                    id: id.to_string(),
                    message_index: Some(message_index),
                    media_index: Some(part_idx as u64),
                    extension: "txt".to_string(),
                    content: text.into_bytes(),
                };
                log_refs.push(LogReference::new(log_file.path()));
                files.push(log_file);
            }
            (SimpleContentLog::Parts(log_refs), files)
        }
    }
}
