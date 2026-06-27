use crate::state::{FileStateCache, FileStateEntry};
use crate::util;
use tokio::fs;

#[derive(Debug, serde::Serialize)]
pub struct WriteFileOutput {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub content: String,
    #[serde(rename = "structuredPatch")]
    pub structured_patch: Vec<util::StructuredPatchHunk>,
    #[serde(rename = "originalFile")]
    pub original_file: Option<String>,
}

pub async fn write_file(
    file_state: &FileStateCache,
    path: &str,
    content: &str,
) -> Result<String, String> {
    // UNC path security check
    if util::is_unc_path(path) {
        return Err("Cannot write files on UNC paths.".into());
    }

    let absolute_path = util::normalize_path_allow_missing(path)
        .await
        .map_err(|e| format!("Failed to resolve path: {e}"))?;
    let absolute_path_str = absolute_path.to_string_lossy().to_string();

    // Check if file exists
    let file_exists = fs::try_exists(&absolute_path).await.unwrap_or(false);

    if file_exists {
        // Must-read check (error code 2)
        let cached = file_state.get(&absolute_path_str).await;
        match cached {
            None => {
                return Err("File has not been read yet. Read it first before writing to it.".into());
            }
            Some(entry) if entry.is_partial_view() => {
                return Err("File has not been read yet. Read it first before writing to it.".into());
            }
            Some(entry) => {
                // Staleness check (error code 3)
                let current_mtime = util::get_file_mtime_ms(&absolute_path)
                    .await
                    .map_err(|e| format!("Failed to get file mtime: {e}"))?;
                if current_mtime > entry.timestamp {
                    // Windows content-comparison fallback for full reads
                    let is_full_read = !entry.is_partial_view && entry.offset.is_none() && entry.limit.is_none();
                    if is_full_read {
                        if let Ok(current_content) = fs::read_to_string(&absolute_path).await {
                            let normalized_current = util::normalize_line_endings(&current_content);
                            let normalized_cached = util::normalize_line_endings(&entry.content);
                            if normalized_current != normalized_cached {
                                return Err(
                                    "File has been modified since read, either by the user or by a linter. Read it again before attempting to write it.".into()
                                );
                            }
                            // Content unchanged despite mtime bump — allow write
                        } else {
                            return Err(
                                "File has been modified since read, either by the user or by a linter. Read it again before attempting to write it.".into()
                            );
                        }
                    } else {
                        return Err(
                            "File has been modified since read, either by the user or by a linter. Read it again before attempting to write it.".into()
                        );
                    }
                }
            }
        }
    }

    // Read original content before overwriting (for patch generation)
    let original_file = if file_exists {
        fs::read_to_string(&absolute_path).await.ok()
    } else {
        None
    };

    // Create parent directories if needed
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directories: {e}"))?;
    }

    // Write the file
    fs::write(&absolute_path, content)
        .await
        .map_err(|e| format!("Failed to write file: {e}"))?;

    // Update readFileState
    let mtime_ms = util::get_file_mtime_ms(&absolute_path)
        .await
        .map_err(|e| format!("Failed to get file mtime: {e}"))?;
    file_state.set(absolute_path_str.clone(), FileStateEntry {
        content: util::normalize_line_endings(content),
        timestamp: mtime_ms,
        offset: None,
        limit: None,
        is_partial_view: false,
    }).await;

    let patch = if let Some(ref orig) = original_file {
        util::make_patch(orig, content)
    } else {
        vec![]
    };

    let output = WriteFileOutput {
        kind: if original_file.is_some() { "update" } else { "create" }.into(),
        file_path: absolute_path_str,
        content: content.to_owned(),
        structured_patch: patch,
        original_file,
    };

    serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize output: {e}"))
}
