use crate::state::{FileStateCache, FileStateEntry};
use crate::util;
use tokio::fs;

const MAX_EDIT_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1 GiB

#[derive(Debug, serde::Serialize)]
pub struct EditFileOutput {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "oldString")]
    pub old_string: String,
    #[serde(rename = "newString")]
    pub new_string: String,
    #[serde(rename = "originalFile")]
    pub original_file: String,
    #[serde(rename = "structuredPatch")]
    pub structured_patch: Vec<util::StructuredPatchHunk>,
    #[serde(rename = "replaceAll")]
    pub replace_all: bool,
    #[serde(rename = "userModified")]
    pub user_modified: bool,
}

pub async fn edit_file(
    file_state: &FileStateCache,
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, String> {
    // UNC path security check
    if util::is_unc_path(path) {
        return Err("Cannot edit files on UNC paths.".into());
    }

    // Reject Jupyter notebook files
    if path.ends_with(".ipynb") {
        return Err("Cannot edit Jupyter Notebook files with the Edit tool. Use NotebookEdit instead.".into());
    }

    // Desanitize inputs (reverse tokenizer sanitization, matching Claude Code)
    let old_string = &util::desanitize(old_string);
    let new_string = &util::strip_trailing_whitespace(&util::desanitize(new_string));

    // Error code 1: no-op edit
    if old_string == new_string {
        return Err("No changes to make: old_string and new_string are exactly the same.".into());
    }

    let absolute_path = util::normalize_path_allow_missing(path)
        .await
        .map_err(|e| format!("Failed to resolve path: {e}"))?;
    let absolute_path_str = absolute_path.to_string_lossy().to_string();
    let absolute_path_exists = fs::try_exists(&absolute_path).await.unwrap_or(false);

    // Fix 8: Empty old_string + file doesn't exist = file creation
    if old_string.is_empty() && !absolute_path_exists {
        if let Some(parent) = absolute_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directories: {e}"))?;
        }
        fs::write(&absolute_path, new_string)
            .await
            .map_err(|e| format!("Failed to create file: {e}"))?;

        // Update cache
        let mtime_ms = util::get_file_mtime_ms(&absolute_path)
            .await
            .map_err(|e| format!("Failed to get file mtime: {e}"))?;
        file_state.set(absolute_path_str.clone(), FileStateEntry {
            content: util::normalize_line_endings(new_string),
            timestamp: mtime_ms,
            offset: None,
            limit: None,
            is_partial_view: false,
        }).await;

        let patch = util::make_patch("", new_string);

        let output = EditFileOutput {
            file_path: absolute_path_str,
            old_string: old_string.to_owned(),
            new_string: new_string.to_owned(),
            original_file: String::new(),
            structured_patch: patch,
            replace_all,
            user_modified: false,
        };

        return serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize output: {e}"));
    }

    // Empty old_string on existing file: allow if file is empty, reject if file has content
    let is_empty_old_on_existing = old_string.is_empty() && absolute_path_exists;
    if is_empty_old_on_existing {
        let existing = fs::read_to_string(&absolute_path).await.unwrap_or_default();
        if !existing.trim().is_empty() {
            return Err("Cannot create new file - file already exists.".into());
        }
        // File is empty — write new_string directly, skip must-read check
        fs::write(&absolute_path, new_string)
            .await
            .map_err(|e| format!("Failed to write file: {e}"))?;
        let mtime_ms = util::get_file_mtime_ms(&absolute_path)
            .await
            .map_err(|e| format!("Failed to get file mtime: {e}"))?;
        file_state.set(absolute_path_str.clone(), FileStateEntry {
            content: util::normalize_line_endings(new_string),
            timestamp: mtime_ms,
            offset: None,
            limit: None,
            is_partial_view: false,
        }).await;
        let patch = util::make_patch("", new_string);
        let output = EditFileOutput {
            file_path: absolute_path_str,
            old_string: old_string.to_owned(),
            new_string: new_string.to_owned(),
            original_file: String::new(),
            structured_patch: patch,
            replace_all,
            user_modified: false,
        };
        return serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize output: {e}"));
    }

    // Error code 4: file does not exist
    if !absolute_path_exists {
        let mut msg = format!(
            "File does not exist. Note: your current working directory is {}.",
            std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
        if let Some(similar) = util::find_similar_file(&absolute_path).await {
            msg.push_str(&format!("\nDid you mean: {similar}"));
        }
        if let Some(suggested) = util::suggest_path_under_cwd(path).await {
            msg.push_str(&format!("\nSuggested path: {suggested}"));
        }
        return Err(msg);
    }

    // Must-read check (error code 6)
    let cached = file_state.get(&absolute_path_str).await;
    match &cached {
        None => {
            return Err("File has not been read yet. Read it first before writing to it.".into());
        }
        Some(entry) if entry.is_partial_view() => {
            return Err("File has not been read yet. Read it first before writing to it.".into());
        }
        Some(_) => {}
    }
    let cached = cached.unwrap();

    // Check file size before reading
    let metadata = fs::metadata(&absolute_path)
        .await
        .map_err(|e| format!("Failed to read file metadata: {e}"))?;
    let file_size = metadata.len();
    if file_size > MAX_EDIT_FILE_SIZE {
        return Err(format!(
            "File is too large to edit ({file_size} bytes exceeds 1 GiB limit)."
        ));
    }

    // Read current file content (bytes for UTF-16LE BOM detection)
    let file_bytes = fs::read(&absolute_path)
        .await
        .map_err(|e| format!("Failed to read file: {e}"))?;
    let (original_file_raw, is_utf16le) = if file_bytes.len() >= 2 && file_bytes[0] == 0xFF && file_bytes[1] == 0xFE {
        // UTF-16LE with BOM
        let u16_iter = file_bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
        let decoded = char::decode_utf16(u16_iter).map(|r| r.unwrap_or('\u{FFFD}')).collect::<String>();
        (decoded, true)
    } else {
        (String::from_utf8_lossy(&file_bytes).into_owned(), false)
    };
    let original_file = util::normalize_line_endings(&original_file_raw);

    // Staleness check (error code 7)
    let current_mtime = util::get_file_mtime_ms(&absolute_path)
        .await
        .map_err(|e| format!("Failed to get file mtime: {e}"))?;
    if current_mtime > cached.timestamp {
        // Windows content-comparison fallback for full reads
        let is_full_read = cached.offset.is_none() && cached.limit.is_none();
        if !(is_full_read && original_file == cached.content) {
            return Err(
                "File has been modified since read, either by the user or by a linter. Read it again before attempting to write it.".into()
            );
        }
    }

    // Find old_string in file with quote normalization fallback
    let actual_old_string = if original_file.contains(old_string) {
        old_string.to_owned()
    } else {
        // Try curly quote normalization
        match util::find_match_with_quote_normalization(&original_file, old_string) {
            Some(matched) => matched.to_owned(),
            None => {
                // Error code 8: match not found
                return Err(format!("String to replace not found in file.\nString: {old_string}"));
            }
        }
    };

    // Count matches
    let matches = original_file.matches(&actual_old_string).count();

    // Error code 9: ambiguous match
    if matches > 1 && !replace_all {
        return Err(format!(
            "Found {matches} matches of the string to replace, but replace_all is false. \
             To replace all occurrences, set replace_all to true. To replace only one \
             occurrence, please provide more context to uniquely identify the instance.\n\
             String: {old_string}"
        ));
    }

    // Preserve the file's curly quote style in new_string when old_string matched via normalization
    let actual_new_string = util::preserve_quote_style(old_string, &actual_old_string, new_string);

    // Apply the edit
    let updated = util::apply_edit(&original_file, &actual_old_string, &actual_new_string, replace_all);

    // Write the updated file (re-encode to UTF-16LE if original was UTF-16LE)
    if is_utf16le {
        let mut out_bytes = vec![0xFF, 0xFE]; // BOM
        for u in updated.encode_utf16() {
            out_bytes.extend_from_slice(&u.to_le_bytes());
        }
        fs::write(&absolute_path, &out_bytes)
            .await
            .map_err(|e| format!("Failed to write file: {e}"))?;
    } else {
        fs::write(&absolute_path, &updated)
            .await
            .map_err(|e| format!("Failed to write file: {e}"))?;
    }

    // Update readFileState
    let mtime_ms = util::get_file_mtime_ms(&absolute_path)
        .await
        .map_err(|e| format!("Failed to get file mtime: {e}"))?;
    file_state.set(absolute_path_str.clone(), FileStateEntry {
        content: updated.clone(),
        timestamp: mtime_ms,
        offset: None,
        limit: None,
        is_partial_view: false,
    }).await;

    let patch = util::make_patch(&original_file, &updated);

    let output = EditFileOutput {
        file_path: absolute_path_str,
        old_string: actual_old_string,
        new_string: new_string.to_owned(),
        original_file,
        structured_patch: patch,
        replace_all,
        user_modified: false,
    };

    serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize output: {e}"))
}
