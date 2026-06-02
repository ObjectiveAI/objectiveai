//! Free-function port of `InputValue::extract_to_files`.

use indexmap::IndexMap;
use objectiveai_sdk::functions::expression::{InputValue, InputValueLog};

use crate::filesystem::logs::{LogFile, LogReference};

/// Recursively extract an `InputValue` tree to per-leaf files,
/// returning the [`InputValueLog`] (with references in place of values)
/// plus every [`LogFile`] the caller writes.
///
/// Leaf variants (String, Integer, Number, Boolean, RichContentPart)
/// each become one file at
/// `<route_base>/input/<dotted-key-path>/<id>.<ext>`; Object / Array
/// children get their own per-value files whose contents are the
/// recursively-extracted [`InputValueLog`] for that sub-tree.
///
/// `key_path` is the dotted-key path from the root (e.g. `""` for the
/// top level, `"foo"`, `"foo.0"`, `"foo.0.bar"`). It's folded into the
/// on-disk path via `/`-substitution so the filesystem layout mirrors
/// the input tree.
pub fn extract_to_files(
    value: InputValue,
    route_base: &str,
    id: &str,
    key_path: &str,
) -> (InputValueLog, Vec<LogFile>) {
    // Dotted key path → slash-separated subdir under `input/`.
    // Empty key_path → "input"; non-empty → "input/<slashed>".
    let input_route = if key_path.is_empty() {
        format!("{route_base}/input")
    } else {
        format!("{route_base}/input/{}", key_path.replace('.', "/"))
    };

    match value {
        InputValue::Object(map) => {
            let mut log_map = IndexMap::with_capacity(map.len());
            let mut all_files: Vec<LogFile> = Vec::new();
            for (key, value) in map {
                let child_key_path = if key_path.is_empty() {
                    key.clone()
                } else {
                    format!("{key_path}.{key}")
                };
                let (child_log, child_files) =
                    extract_to_files(value, route_base, id, &child_key_path);
                all_files.extend(child_files);
                let child_route = format!(
                    "{route_base}/input/{}",
                    child_key_path.replace('.', "/")
                );
                let child_file = LogFile {
                    route: child_route,
                    id: id.to_string(),
                    message_index: None,
                    media_index: None,
                    extension: "json".to_string(),
                    content: serde_json::to_vec_pretty(&child_log)
                        .expect("InputValueLog serializes"),
                };
                log_map.insert(key, LogReference::new(child_file.path()));
                all_files.push(child_file);
            }
            (InputValueLog::Object(log_map), all_files)
        }
        InputValue::Array(arr) => {
            let mut log_vec = Vec::with_capacity(arr.len());
            let mut all_files: Vec<LogFile> = Vec::new();
            for (idx, value) in arr.into_iter().enumerate() {
                let child_key_path = if key_path.is_empty() {
                    idx.to_string()
                } else {
                    format!("{key_path}.{idx}")
                };
                let (child_log, child_files) =
                    extract_to_files(value, route_base, id, &child_key_path);
                all_files.extend(child_files);
                let child_route = format!(
                    "{route_base}/input/{}",
                    child_key_path.replace('.', "/")
                );
                let child_file = LogFile {
                    route: child_route,
                    id: id.to_string(),
                    message_index: None,
                    media_index: None,
                    extension: "json".to_string(),
                    content: serde_json::to_vec_pretty(&child_log)
                        .expect("InputValueLog serializes"),
                };
                log_vec.push(LogReference::new(child_file.path()));
                all_files.push(child_file);
            }
            (InputValueLog::Array(log_vec), all_files)
        }
        // Leaf variants — each writes one file with its native
        // representation; the InputValueLog at the parent level holds
        // a Reference to it.
        InputValue::String(s) => {
            let file = LogFile {
                route: input_route.clone(),
                id: id.to_string(),
                message_index: None,
                media_index: None,
                extension: "txt".to_string(),
                content: s.into_bytes(),
            };
            let r = LogReference::new(file.path());
            (InputValueLog::Reference(r), vec![file])
        }
        ref leaf @ (InputValue::Integer(_)
        | InputValue::Number(_)
        | InputValue::Boolean(_)
        | InputValue::RichContentPart(_)) => {
            let file = LogFile {
                route: input_route.clone(),
                id: id.to_string(),
                message_index: None,
                media_index: None,
                extension: "json".to_string(),
                content: serde_json::to_vec_pretty(leaf)
                    .expect("InputValue leaf serializes"),
            };
            let r = LogReference::new(file.path());
            (InputValueLog::Reference(r), vec![file])
        }
    }
}
