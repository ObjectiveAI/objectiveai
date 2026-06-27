use std::path::Path;
use std::time::{Instant, SystemTime};

use tokio::fs;

use crate::util;

#[derive(Debug, serde::Serialize)]
pub struct GlobSearchOutput {
    #[serde(rename = "durationMs")]
    pub duration_ms: u128,
    #[serde(rename = "numFiles")]
    pub num_files: usize,
    pub filenames: Vec<String>,
    pub truncated: bool,
}

pub async fn glob_search(pattern: &str, path: Option<&str>) -> Result<String, String> {
    let started = Instant::now();

    let base_dir = match path {
        Some(p) => util::normalize_path(p).await.map_err(|e| format!("Invalid path: {e}"))?,
        None => std::env::current_dir().map_err(|e| format!("Failed to get CWD: {e}"))?,
    };

    let search_pattern = if Path::new(pattern).is_absolute() {
        pattern.to_owned()
    } else {
        base_dir.join(pattern).to_string_lossy().into_owned()
    };

    // The `glob` crate's iterator is sync, but only inspects directory entries
    // (no file content) — collect candidate paths first, then probe each
    // with tokio::fs::metadata to filter to regular files and capture mtimes.
    let candidates: Vec<std::path::PathBuf> = glob::glob(&search_pattern)
        .map_err(|e| format!("Invalid glob pattern: {e}"))?
        .flatten()
        .collect();

    let mut matches: Vec<(std::path::PathBuf, Option<SystemTime>)> = Vec::new();
    for entry in candidates {
        let metadata = match fs::metadata(&entry).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !metadata.is_file() {
            continue;
        }
        let mtime = metadata.modified().ok();
        matches.push((entry, mtime));
    }

    // Sort by modification time, oldest first (ascending mtime)
    matches.sort_by(|a, b| a.1.cmp(&b.1));

    let truncated = matches.len() > 100;
    let filenames: Vec<String> = matches
        .into_iter()
        .take(100)
        .map(|(p, _)| p.to_string_lossy().into_owned())
        .collect();

    let output = GlobSearchOutput {
        duration_ms: started.elapsed().as_millis(),
        num_files: filenames.len(),
        filenames,
        truncated,
    };

    serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize output: {e}"))
}
