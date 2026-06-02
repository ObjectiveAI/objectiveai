//! Free-function ports of the SDK's filesystem-coupled `extract_media`
//! methods on `SimpleContent` and `RichContent`. These used to live on
//! the SDK types as `pub fn extract_media(self, ...)` gated on the now-
//! deleted `filesystem` feature; since `LogFile` and `LogReference`
//! belong to the CLI, the constructors live here as free functions per
//! `feedback_extract_methods_relocate_to_cli.md`.

use objectiveai_sdk::agent::completions::message::{
    RichContent, RichContentLog, RichContentPart, SimpleContent,
    SimpleContentLog, SimpleContentPart,
};

use super::{LogFile, LogReference};

/// Extract every chunk of a `SimpleContent` into its own on-disk log
/// file. Parallel to [`extract_rich_content_media`].
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
pub fn extract_simple_content_media(
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

/// Extract every chunk of a `RichContent` into its own on-disk log
/// file, returning a [`RichContentLog`] of [`LogReference`]s pointing
/// at the written files (no inline content — the log is purely
/// references).
///
/// Per-chunk write rules:
///
/// - `RichContent::Text(text)` → one `.txt` file at
///   `<media_root>/text/<id>-<idx>.txt` containing the raw UTF-8 text.
///   Returns `RichContentLog::Reference(ref)`.
/// - `RichContent::Parts(parts)` → one file per part (rules in
///   [`extract_one_rich_part`]). Returns `RichContentLog::Parts(refs)`
///   in original order.
///
/// `media_root` is the parent directory under which the per-media-type
/// subdirs (`text`, `image`, `audio`, `video`, `file`) get created.
/// Callers pass things like `"agents/completions/request/messages"` for
/// message extraction or `"agents/completions/request/notifications"`
/// for notification extraction.
///
/// `id` and `message_index` identify the parent record.
pub fn extract_rich_content_media(
    content: RichContent,
    media_root: &str,
    id: &str,
    message_index: u64,
) -> (RichContentLog, Vec<LogFile>) {
    match content {
        RichContent::Text(text) => {
            let log_file = LogFile {
                route: format!("{media_root}/text"),
                id: id.to_string(),
                message_index: Some(message_index),
                media_index: None,
                extension: "txt".to_string(),
                content: text.into_bytes(),
            };
            let reference = LogReference::new(log_file.path());
            (RichContentLog::Reference(reference), vec![log_file])
        }
        RichContent::Parts(parts) => {
            let mut log_refs = Vec::with_capacity(parts.len());
            let mut files = Vec::with_capacity(parts.len());
            for (part_idx, part) in parts.into_iter().enumerate() {
                let file = extract_one_rich_part(
                    part,
                    media_root,
                    id,
                    message_index,
                    part_idx as u64,
                );
                log_refs.push(LogReference::new(file.path()));
                files.push(file);
            }
            (RichContentLog::Parts(log_refs), files)
        }
    }
}

/// Write one `RichContentPart` to its own [`LogFile`].
///
/// File-type choice per part:
/// - `Text { text }` → `.txt` (raw UTF-8) under `<media_root>/text/`.
/// - Inline-decodable media (`ImageUrl` / `InputAudio` / `InputVideo` /
///   `VideoUrl` / `File` whose `file_content()` yields a `FileContent`
///   that successfully `decode()`s) → the decoded binary written under
///   `<media_root>/<media_dir>/` with the `FileContent`'s native
///   extension.
/// - Anything else (remote URLs, non-decodable inline data) →
///   `serde_json::to_vec_pretty(&part)` written under
///   `<media_root>/<media_dir>/` as a `.json` file. The reader can
///   `serde_json::from_slice::<RichContentPart>` it back.
///
/// `media_dir` is debug grouping only — `image|audio|video|file` per
/// variant. Not load-bearing for parsing (the reader keys off the file
/// extension).
fn extract_one_rich_part(
    part: RichContentPart,
    media_root: &str,
    id: &str,
    message_index: u64,
    part_idx: u64,
) -> LogFile {
    // Text branches out early — never goes through file_content().
    if let RichContentPart::Text { text } = &part {
        return LogFile {
            route: format!("{media_root}/text"),
            id: id.to_string(),
            message_index: Some(message_index),
            media_index: Some(part_idx),
            extension: "txt".to_string(),
            content: text.clone().into_bytes(),
        };
    }

    let (media_dir, bin_attempt) = match &part {
        RichContentPart::Text { .. } => unreachable!("handled above"),
        RichContentPart::ImageUrl { image_url } => {
            ("image", image_url.file_content())
        }
        RichContentPart::InputAudio { input_audio } => {
            ("audio", input_audio.file_content())
        }
        RichContentPart::InputVideo { video_url }
        | RichContentPart::VideoUrl { video_url } => {
            ("video", video_url.file_content())
        }
        RichContentPart::File { file } => ("file", file.file_content()),
    };

    if let Some(fc) = bin_attempt {
        if let Ok(decoded) = fc.decode() {
            return LogFile {
                route: format!("{media_root}/{media_dir}"),
                id: id.to_string(),
                message_index: Some(message_index),
                media_index: Some(part_idx),
                extension: fc.extension.to_string(),
                content: decoded,
            };
        }
    }

    // Fallback: serialize the part itself (covers remote URLs +
    // any non-decodable inline data).
    let json = serde_json::to_vec_pretty(&part)
        .expect("RichContentPart serializes");
    LogFile {
        route: format!("{media_root}/{media_dir}"),
        id: id.to_string(),
        message_index: Some(message_index),
        media_index: Some(part_idx),
        extension: "json".to_string(),
        content: json,
    }
}
