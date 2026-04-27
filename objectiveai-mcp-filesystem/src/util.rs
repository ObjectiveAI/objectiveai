use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tokio::fs;

/// Check if a path is a UNC path (\\server\share or //server/share).
/// UNC paths on Windows can trigger SMB authentication and leak NTLM credentials.
pub fn is_unc_path(path: &str) -> bool {
    path.starts_with("\\\\") || path.starts_with("//")
}

/// Search for a file with the same base name but different extension in the same directory.
/// Returns the full path to the similar file, matching Claude Code's findSimilarFile behavior.
pub async fn find_similar_file(path: &Path) -> Option<String> {
    let parent = path.parent()?;
    let stem = path.file_stem()?.to_str()?;
    let mut entries = fs::read_dir(parent).await.ok()?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_path = entry.path();
        let is_file = fs::metadata(&entry_path)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false);
        if is_file && entry_path != path {
            if let Some(entry_stem) = entry_path.file_stem().and_then(|s| s.to_str()) {
                if entry_stem == stem {
                    return Some(entry_path.to_string_lossy().into_owned());
                }
            }
        }
    }
    None
}

/// Check if the requested path exists under the current working directory.
/// Handles "dropped repo folder" pattern where user requests /full/path/to/file
/// but the file actually exists at $CWD/relative/path.
/// Matches Claude Code's suggestPathUnderCwd: collects path components,
/// then tries progressively shorter suffixes (from 1 component to all) under CWD.
pub async fn suggest_path_under_cwd(requested_path: &str) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let requested = Path::new(requested_path);
    let components: Vec<std::path::Component> = requested.components().collect();
    if components.is_empty() {
        return None;
    }

    // Try suffixes from shortest (just filename) to longest (all but first component).
    // This matches Claude Code which iterates i from components.length-1 down to 1.
    for i in (1..components.len()).rev() {
        let suffix: PathBuf = components[i..].iter().collect();
        let candidate = cwd.join(&suffix);
        if fs::try_exists(&candidate).await.unwrap_or(false) {
            // Make sure we're not just returning the same path
            let candidate_canonical = fs::canonicalize(&candidate).await.ok();
            let requested_canonical = fs::canonicalize(requested).await.ok();
            if candidate_canonical != requested_canonical {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Normalize a path to an absolute canonical path.
pub async fn normalize_path(path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };
    fs::canonicalize(&candidate).await
}

/// Normalize a path, allowing the file to not exist yet (for writes).
pub async fn normalize_path_allow_missing(path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };

    if let Ok(canonical) = fs::canonicalize(&candidate).await {
        return Ok(canonical);
    }

    if let Some(parent) = candidate.parent() {
        let canonical_parent = fs::canonicalize(parent)
            .await
            .unwrap_or_else(|_| parent.to_path_buf());
        if let Some(name) = candidate.file_name() {
            return Ok(canonical_parent.join(name));
        }
    }

    Ok(candidate)
}

/// Get file modification time in milliseconds since epoch.
pub async fn get_file_mtime_ms(path: &Path) -> io::Result<u64> {
    let metadata = fs::metadata(path).await?;
    let modified = metadata.modified()?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64)
}

/// Normalize line endings: CRLF → LF.
pub fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n")
}

/// Blocked device paths that should not be read.
const BLOCKED_DEVICES: &[&str] = &[
    "/dev/zero",
    "/dev/random",
    "/dev/urandom",
    "/dev/full",
    "/dev/stdin",
    "/dev/tty",
    "/dev/console",
    "/dev/stdout",
    "/dev/stderr",
    "/dev/fd/0",
    "/dev/fd/1",
    "/dev/fd/2",
];

/// Check if a path is a blocked device.
pub fn is_blocked_device(path: &str) -> bool {
    BLOCKED_DEVICES.iter().any(|d| path == *d)
        || path.starts_with("/proc/") && path.contains("/fd/")
}

/// Simple structured patch hunk for diff output.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StructuredPatchHunk {
    #[serde(rename = "oldStart")]
    pub old_start: usize,
    #[serde(rename = "oldLines")]
    pub old_lines: usize,
    #[serde(rename = "newStart")]
    pub new_start: usize,
    #[serde(rename = "newLines")]
    pub new_lines: usize,
    pub lines: Vec<String>,
}

/// Generate a structured patch between two strings using line-level diff.
pub fn make_patch(original: &str, updated: &str) -> Vec<StructuredPatchHunk> {
    let old_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = updated.lines().collect();

    if old_lines == new_lines {
        return Vec::new();
    }

    // Find first differing line
    let common_prefix = old_lines
        .iter()
        .zip(new_lines.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Find last differing line (from end)
    let common_suffix = old_lines
        .iter()
        .rev()
        .zip(new_lines.iter().rev())
        .take_while(|(a, b)| a == b)
        .count()
        .min(old_lines.len() - common_prefix)
        .min(new_lines.len() - common_prefix);

    let old_changed_end = old_lines.len() - common_suffix;
    let new_changed_end = new_lines.len() - common_suffix;

    // Build hunk with up to 3 lines of context
    let ctx = 3;
    let hunk_old_start = common_prefix.saturating_sub(ctx);
    let hunk_old_end = (old_changed_end + ctx).min(old_lines.len());
    let hunk_new_start = common_prefix.saturating_sub(ctx);
    let hunk_new_end = (new_changed_end + ctx).min(new_lines.len());

    let mut lines = Vec::new();

    // Context before
    for i in hunk_old_start..common_prefix {
        lines.push(format!(" {}", old_lines[i]));
    }
    // Removed lines
    for i in common_prefix..old_changed_end {
        lines.push(format!("-{}", old_lines[i]));
    }
    // Added lines
    for i in common_prefix..new_changed_end {
        lines.push(format!("+{}", new_lines[i]));
    }
    // Context after
    for i in old_changed_end..hunk_old_end {
        lines.push(format!(" {}", old_lines[i]));
    }

    vec![StructuredPatchHunk {
        old_start: hunk_old_start + 1, // 1-indexed
        old_lines: hunk_old_end - hunk_old_start,
        new_start: hunk_new_start + 1, // 1-indexed
        new_lines: hunk_new_end - hunk_new_start,
        lines,
    }]
}

/// Curly quote characters for normalization.
const LEFT_SINGLE_CURLY: char = '\u{2018}';
const RIGHT_SINGLE_CURLY: char = '\u{2019}';
const LEFT_DOUBLE_CURLY: char = '\u{201C}';
const RIGHT_DOUBLE_CURLY: char = '\u{201D}';

/// Try to find old_string in content, falling back to curly quote normalization.
/// Returns the actual matched string from the file content.
pub fn find_match_with_quote_normalization<'a>(content: &'a str, search: &str) -> Option<&'a str> {
    // Try exact match first
    if let Some(idx) = content.find(search) {
        return Some(&content[idx..idx + search.len()]);
    }

    // Normalize both search and content: curly quotes → straight quotes
    let normalized_search = search
        .replace(LEFT_SINGLE_CURLY, "'")
        .replace(RIGHT_SINGLE_CURLY, "'")
        .replace(LEFT_DOUBLE_CURLY, "\"")
        .replace(RIGHT_DOUBLE_CURLY, "\"");

    let normalized_content = content
        .replace(LEFT_SINGLE_CURLY, "'")
        .replace(RIGHT_SINGLE_CURLY, "'")
        .replace(LEFT_DOUBLE_CURLY, "\"")
        .replace(RIGHT_DOUBLE_CURLY, "\"");

    // If neither had curly quotes, no normalization match possible
    if normalized_search == search && normalized_content == content {
        return None;
    }

    // Try finding the normalized search in the normalized content.
    // Then map the position back to the original content.
    if let Some(norm_byte_idx) = normalized_content.find(&normalized_search) {
        // Count the number of characters before the match in normalized content
        let char_offset = normalized_content[..norm_byte_idx].chars().count();
        let match_char_len = normalized_search.chars().count();

        // Map character offset back to byte offset in the original content
        let orig_byte_start = content
            .char_indices()
            .nth(char_offset)
            .map(|(i, _)| i)?;
        let orig_byte_end = content
            .char_indices()
            .nth(char_offset + match_char_len)
            .map(|(i, _)| i)
            .unwrap_or(content.len());

        Some(&content[orig_byte_start..orig_byte_end])
    } else {
        None
    }
}

/// When old_string matched via quote normalization (curly quotes in file,
/// straight quotes from model), apply the same curly quote style to new_string
/// so the edit preserves the file's typography.
pub fn preserve_quote_style(old_string: &str, actual_old_string: &str, new_string: &str) -> String {
    // If they're the same, no normalization happened
    if old_string == actual_old_string {
        return new_string.to_owned();
    }

    // Detect which curly quote types were in the file's matched text
    let has_double = actual_old_string.contains(LEFT_DOUBLE_CURLY)
        || actual_old_string.contains(RIGHT_DOUBLE_CURLY);
    let has_single = actual_old_string.contains(LEFT_SINGLE_CURLY)
        || actual_old_string.contains(RIGHT_SINGLE_CURLY);

    if !has_double && !has_single {
        return new_string.to_owned();
    }

    let mut result = new_string.to_owned();
    if has_double {
        result = apply_curly_double_quotes(&result);
    }
    if has_single {
        result = apply_curly_single_quotes(&result);
    }
    result
}

/// Returns true if the character at `index` in `chars` is in an opening-quote context.
fn is_opening_context(chars: &[char], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    matches!(
        chars[index - 1],
        ' ' | '\t' | '\n' | '\r' | '(' | '[' | '{' | '\u{2014}' | '\u{2013}'
    )
}

/// Replace straight double quotes with curly double quotes using open/close heuristics.
fn apply_curly_double_quotes(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::with_capacity(s.len());
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '"' {
            if is_opening_context(&chars, i) {
                result.push(LEFT_DOUBLE_CURLY);
            } else {
                result.push(RIGHT_DOUBLE_CURLY);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Replace straight single quotes with curly single quotes using open/close heuristics.
/// Apostrophes in contractions (letter-'-letter) get right single curly quote.
fn apply_curly_single_quotes(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::with_capacity(s.len());
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '\'' {
            let prev_is_letter = i > 0 && chars[i - 1].is_alphabetic();
            let next_is_letter = i + 1 < chars.len() && chars[i + 1].is_alphabetic();
            if prev_is_letter && next_is_letter {
                // Apostrophe in a contraction — use right single curly quote
                result.push(RIGHT_SINGLE_CURLY);
            } else if is_opening_context(&chars, i) {
                result.push(LEFT_SINGLE_CURLY);
            } else {
                result.push(RIGHT_SINGLE_CURLY);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Apply an edit to file content.
pub fn apply_edit(
    original: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> String {
    if new_string.is_empty() {
        // When deleting (new_string is empty), strip trailing newline after each match
        let strip_trailing =
            !old_string.ends_with('\n') && original.contains(&format!("{old_string}\n"));
        let search = if strip_trailing {
            format!("{old_string}\n")
        } else {
            old_string.to_string()
        };
        if replace_all {
            original.replace(&search, "")
        } else {
            original.replacen(&search, "", 1)
        }
    } else if replace_all {
        original.replace(old_string, new_string)
    } else {
        original.replacen(old_string, new_string, 1)
    }
}

/// Strip trailing whitespace from each line while preserving line endings.
/// Matches Claude Code's stripTrailingWhitespace behavior.
pub fn strip_trailing_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut line_start = 0;
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\r' && i + 1 < len && bytes[i + 1] == b'\n' {
            // CRLF
            let line = &s[line_start..i];
            result.push_str(line.trim_end());
            result.push_str("\r\n");
            i += 2;
            line_start = i;
        } else if bytes[i] == b'\n' || bytes[i] == b'\r' {
            // LF or CR
            let line = &s[line_start..i];
            result.push_str(line.trim_end());
            result.push(bytes[i] as char);
            i += 1;
            line_start = i;
        } else {
            i += 1;
        }
    }
    // Last line (no trailing newline)
    if line_start < len {
        result.push_str(s[line_start..].trim_end());
    }
    result
}

/// Desanitization replacements matching Claude Code's DESANITIZATIONS constant.
/// These reverse sanitization that the model's tokenizer may apply to certain XML-like tags.
const DESANITIZATIONS: &[(&str, &str)] = &[
    ("<fnr>", "<function_results>"),
    ("<n>", "<name>"),
    ("</n>", "</name>"),
    ("<o>", "<output>"),
    ("</o>", "</output>"),
    ("<e>", "<error>"),
    ("</e>", "</error>"),
    ("<s>", "<system>"),
    ("</s>", "</system>"),
    ("<r>", "<result>"),
    ("</r>", "</result>"),
    ("< META_START >", "<META_START>"),
    ("< META_END >", "<META_END>"),
    ("< EOT >", "<EOT>"),
    ("< META >", "<META>"),
    ("< SOS >", "<SOS>"),
    ("\n\nH:", "\n\nHuman:"),
    ("\n\nA:", "\n\nAssistant:"),
];

/// Apply desanitization to a string, reversing tokenizer sanitization.
/// Matches Claude Code's desanitize behavior.
pub fn desanitize(s: &str) -> String {
    let mut result = s.to_owned();
    for &(from, to) in DESANITIZATIONS {
        if result.contains(from) {
            result = result.replace(from, to);
        }
    }
    result
}
