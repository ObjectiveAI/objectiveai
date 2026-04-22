use std::path::PathBuf;

use super::ListItem;

/// Result of reading a log file — either parsed JSON or a data URL.
#[derive(Debug)]
pub enum LogContent {
    Json(serde_json::Value),
    /// A `data:{mime};base64,{payload}` string.
    DataUrl(String),
}

fn endpoint_dir(client: &super::super::Client, endpoint: &str) -> PathBuf {
    let mut dir = client.logs_dir();
    for segment in endpoint.split('/') {
        dir = dir.join(segment);
    }
    dir
}

async fn list_endpoint(client: &super::super::Client, endpoint: &str, offset: usize, limit: usize) -> Result<Vec<ListItem>, super::super::Error> {
    let dir = endpoint_dir(client, endpoint);
    match tokio::fs::metadata(&dir).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(super::super::Error::ReadDir(dir, e)),
        Ok(_) => {}
    }
    let mut read_dir = tokio::fs::read_dir(&dir).await
        .map_err(|e| super::super::Error::ReadDir(dir.clone(), e))?;
    let mut items = Vec::new();
    while let Some(entry) = read_dir.next_entry().await
        .map_err(|e| super::super::Error::ReadDir(dir.clone(), e))?
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let metadata = tokio::fs::metadata(&path).await
            .map_err(|e| super::super::Error::Read(path.clone(), e))?;
        let created = metadata.modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        items.push(ListItem { id: stem, created });
    }
    items.sort_by(|a, b| b.created.cmp(&a.created));
    if offset > 0 || limit < usize::MAX {
        items = items.into_iter().skip(offset).take(limit).collect();
    }
    Ok(items)
}

// -----------------------------------------------------------------------
// List methods
// -----------------------------------------------------------------------

pub async fn list_agent_completions(client: &super::super::Client, offset: usize, limit: usize) -> Result<Vec<ListItem>, super::super::Error> {
    list_endpoint(client, "agents/completions", offset, limit).await
}

pub async fn list_vector_completions(client: &super::super::Client, offset: usize, limit: usize) -> Result<Vec<ListItem>, super::super::Error> {
    list_endpoint(client, "vector/completions", offset, limit).await
}

pub async fn list_function_executions(client: &super::super::Client, offset: usize, limit: usize) -> Result<Vec<ListItem>, super::super::Error> {
    list_endpoint(client, "functions/executions", offset, limit).await
}

pub async fn list_function_inventions(client: &super::super::Client, offset: usize, limit: usize) -> Result<Vec<ListItem>, super::super::Error> {
    list_endpoint(client, "functions/inventions", offset, limit).await
}

pub async fn list_function_inventions_recursive(client: &super::super::Client, offset: usize, limit: usize) -> Result<Vec<ListItem>, super::super::Error> {
    list_endpoint(client, "functions/inventions/recursive", offset, limit).await
}

// pub async fn list_function_profile_computations(client: &super::super::Client, offset: usize, limit: usize) -> Result<Vec<ListItem>, super::super::Error> {
//     list_endpoint(client, "functions/profiles/computations", offset, limit).await
// }

pub async fn list_laboratory_executions(client: &super::super::Client, offset: usize, limit: usize) -> Result<Vec<ListItem>, super::super::Error> {
    list_endpoint(client, "laboratories/executions", offset, limit).await
}

// -----------------------------------------------------------------------
// Clear methods
// -----------------------------------------------------------------------

/// Deletes all files (not subdirectories) in the given endpoint directory.
async fn clear_endpoint(client: &super::super::Client, endpoint: &str) -> Result<u64, super::super::Error> {
    let dir = endpoint_dir(client, endpoint);
    match tokio::fs::metadata(&dir).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(super::super::Error::ReadDir(dir, e)),
        Ok(_) => {}
    }
    let mut read_dir = tokio::fs::read_dir(&dir).await
        .map_err(|e| super::super::Error::ReadDir(dir.clone(), e))?;
    let mut count = 0u64;
    while let Some(entry) = read_dir.next_entry().await
        .map_err(|e| super::super::Error::ReadDir(dir.clone(), e))?
    {
        let path = entry.path();
        if path.is_file() {
            tokio::fs::remove_file(&path).await
                .map_err(|e| super::super::Error::Read(path, e))?;
            count += 1;
        }
    }
    Ok(count)
}

pub async fn clear_agent_completions(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "agents/completions").await
}

pub async fn clear_agent_completion_continuations(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "agent/completions/continuation").await
}

pub async fn clear_agent_completion_messages(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "agent/completions/messages").await
}

pub async fn clear_agent_completion_message_logprobs(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "agent/completions/messages/logprobs").await
}

pub async fn clear_agent_completion_message_images(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "agent/completions/messages/image").await
}

pub async fn clear_agent_completion_message_audio(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "agent/completions/messages/audio").await
}

pub async fn clear_agent_completion_message_video(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "agent/completions/messages/video").await
}

pub async fn clear_agent_completion_message_files(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "agent/completions/messages/file").await
}

pub async fn clear_vector_completions(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "vector/completions").await
}

pub async fn clear_function_executions(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "functions/executions").await
}

pub async fn clear_function_execution_retry_tokens(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "functions/executions/retry_token").await
}

pub async fn clear_function_inventions(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "functions/inventions").await
}

pub async fn clear_function_inventions_recursive(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "functions/inventions/recursive").await
}

pub async fn clear_laboratory_executions(client: &super::super::Client) -> Result<u64, super::super::Error> {
    clear_endpoint(client, "laboratories/executions").await
}

// -----------------------------------------------------------------------
// Write methods
// -----------------------------------------------------------------------

pub fn write_agent_completion(client: &super::super::Client) -> super::LogWriter<crate::agent::completions::response::streaming::AgentCompletionChunk> {
    super::LogWriter::new(client.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
}

pub fn write_vector_completion(client: &super::super::Client) -> super::LogWriter<crate::vector::completions::response::streaming::VectorCompletionChunk> {
    super::LogWriter::new(client.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
}

pub fn write_function_execution(client: &super::super::Client) -> super::LogWriter<crate::functions::executions::response::streaming::FunctionExecutionChunk> {
    super::LogWriter::new(client.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
}

pub fn write_function_invention(client: &super::super::Client) -> super::LogWriter<crate::functions::inventions::response::streaming::FunctionInventionChunk> {
    super::LogWriter::new(client.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
}

pub fn write_function_invention_recursive(client: &super::super::Client) -> super::LogWriter<crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk> {
    super::LogWriter::new(client.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
}

// pub fn write_function_profile_computation(client: &super::super::Client) -> super::LogWriter<crate::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk> {
//     super::LogWriter::new(client.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
// }

pub fn write_laboratory_execution(client: &super::super::Client) -> super::LogWriter<crate::laboratories::executions::response::streaming::LaboratoryExecutionChunk> {
    super::LogWriter::new(client.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
}

// -----------------------------------------------------------------------
// Read helpers
// -----------------------------------------------------------------------

async fn read_json(
    client: &super::super::Client,
    dir: &str,
    stem: &str,
    jq: Option<&str>,
) -> Result<serde_json::Value, super::super::Error> {
    let full = client.logs_dir().join(dir).join(format!("{stem}.json"));
    let bytes = tokio::fs::read(&full).await
        .map_err(|e| super::super::Error::Read(full.clone(), e))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| super::super::Error::Parse(full, e))?;
    apply_jq(value, jq)
}

/// Applies a jq filter to a JSON value, collapsing the multi-result vector
/// the same way the CLI `config get` command does: a single result is
/// unwrapped, an empty result becomes JSON null, and multiple results are
/// wrapped as an array. When `jq` is `None`, the value is returned as-is.
fn apply_jq(
    value: serde_json::Value,
    jq: Option<&str>,
) -> Result<serde_json::Value, super::super::Error> {
    let Some(filter) = jq else { return Ok(value); };
    let mut results = super::super::run_jq(&value, filter)?;
    Ok(match results.len() {
        0 => serde_json::Value::Null,
        1 => results.remove(0),
        _ => serde_json::Value::Array(results),
    })
}

/// Finds the first file in `dir` whose name starts with `stem.` (any extension)
/// and returns it as a data URL.
async fn read_data_url_by_stem(client: &super::super::Client, dir: &str, stem: &str) -> Result<String, super::super::Error> {
    use base64::Engine;
    let dir_path = client.logs_dir().join(dir);
    let prefix = format!("{stem}.");
    let mut read_dir = tokio::fs::read_dir(&dir_path).await
        .map_err(|e| super::super::Error::ReadDir(dir_path.clone(), e))?;
    while let Some(entry) = read_dir.next_entry().await
        .map_err(|e| super::super::Error::ReadDir(dir_path.clone(), e))?
    {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with(&prefix) {
                let bytes = tokio::fs::read(&path).await
                    .map_err(|e| super::super::Error::Read(path.clone(), e))?;
                let mime = mime_guess::from_path(&path)
                    .first_or_octet_stream()
                    .to_string();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                return Ok(format!("data:{mime};base64,{b64}"));
            }
        }
    }
    Err(super::super::Error::Read(dir_path.join(&prefix), std::io::Error::new(std::io::ErrorKind::NotFound, "no matching file")))
}

// -----------------------------------------------------------------------
// Read methods — agent completions
// -----------------------------------------------------------------------

pub async fn read_agent_completion(client: &super::super::Client, id: &str, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "agents/completions", id, jq).await
}

pub async fn read_agent_completion_continuation(client: &super::super::Client, id: &str, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "agents/completions/continuation", id, jq).await
}

pub async fn read_agent_completion_message(client: &super::super::Client, id: &str, message_index: u64, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "agents/completions/messages", &format!("{id}_{message_index}"), jq).await
}

pub async fn read_agent_completion_message_logprobs(client: &super::super::Client, id: &str, message_index: u64, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "agents/completions/messages/logprobs", &format!("{id}_{message_index}"), jq).await
}

pub async fn read_agent_completion_message_image(client: &super::super::Client, id: &str, message_index: u64, media_index: u64) -> Result<String, super::super::Error> {
    read_data_url_by_stem(client, "agents/completions/messages/image", &format!("{id}_{message_index}_{media_index}")).await
}

pub async fn read_agent_completion_message_audio(client: &super::super::Client, id: &str, message_index: u64, media_index: u64) -> Result<String, super::super::Error> {
    read_data_url_by_stem(client, "agents/completions/messages/audio", &format!("{id}_{message_index}_{media_index}")).await
}

pub async fn read_agent_completion_message_video(client: &super::super::Client, id: &str, message_index: u64, media_index: u64) -> Result<String, super::super::Error> {
    read_data_url_by_stem(client, "agents/completions/messages/video", &format!("{id}_{message_index}_{media_index}")).await
}

pub async fn read_agent_completion_message_file(client: &super::super::Client, id: &str, message_index: u64, media_index: u64) -> Result<String, super::super::Error> {
    read_data_url_by_stem(client, "agents/completions/messages/file", &format!("{id}_{message_index}_{media_index}")).await
}

// -----------------------------------------------------------------------
// Read methods — vector completions
// -----------------------------------------------------------------------

pub async fn read_vector_completion(client: &super::super::Client, id: &str, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "vector/completions", id, jq).await
}

// -----------------------------------------------------------------------
// Read methods — function executions
// -----------------------------------------------------------------------

pub async fn read_function_execution(client: &super::super::Client, id: &str, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "functions/executions", id, jq).await
}

pub async fn read_function_execution_retry_token(client: &super::super::Client, id: &str, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "functions/executions/retry_token", id, jq).await
}

// -----------------------------------------------------------------------
// Read methods — function inventions
// -----------------------------------------------------------------------

pub async fn read_function_invention(client: &super::super::Client, id: &str, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "functions/inventions", id, jq).await
}

// -----------------------------------------------------------------------
// Read methods — function inventions recursive
// -----------------------------------------------------------------------

pub async fn read_function_invention_recursive(client: &super::super::Client, id: &str, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "functions/inventions/recursive", id, jq).await
}

// -----------------------------------------------------------------------
// Read methods — laboratory executions
// -----------------------------------------------------------------------

pub async fn read_laboratory_execution(client: &super::super::Client, id: &str, jq: Option<&str>) -> Result<serde_json::Value, super::super::Error> {
    read_json(client, "laboratories/executions", id, jq).await
}

// -----------------------------------------------------------------------
// Subscribe helpers
// -----------------------------------------------------------------------

/// Polls for a JSON file. If `require_modification` is false, returns
/// immediately when the file exists. If true, waits for creation or
/// modification. Returns `Ok(None)` on deletion or timeout. When `jq` is
/// provided, the result is run through the filter before returning.
async fn subscribe_json(
    client: &super::super::Client,
    dir: &str,
    stem: &str,
    timeout: std::time::Duration,
    require_modification: bool,
    jq: Option<&str>,
) -> Result<Option<serde_json::Value>, super::super::Error> {
    let full = client.logs_dir().join(dir).join(format!("{stem}.json"));
    if poll_file(&full, timeout, require_modification).await.is_none() {
        return Ok(None);
    }
    let bytes = match tokio::fs::read(&full).await {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    apply_jq(value, jq).map(Some)
}

/// Polls for a media file (any extension matching `stem.`). If
/// `require_modification` is false, returns immediately when the file
/// exists. If true, waits for creation or modification. Returns `None`
/// on deletion or timeout.
async fn subscribe_data_url_by_stem(
    client: &super::super::Client,
    dir: &str,
    stem: &str,
    timeout: std::time::Duration,
    require_modification: bool,
) -> Option<String> {
    use base64::Engine;
    let dir_path = client.logs_dir().join(dir);
    let prefix = format!("{stem}.");

    let deadline = tokio::time::Instant::now() + timeout;
    let initial_mtime = find_file_mtime_by_prefix(&dir_path, &prefix).await;

    // If file exists and we don't require modification, return immediately
    if !require_modification {
        if let Some((path, _)) = &initial_mtime {
            let bytes = tokio::fs::read(path).await.ok()?;
            let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            return Some(format!("data:{mime};base64,{b64}"));
        }
    }

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if tokio::time::Instant::now() >= deadline {
            return None;
        }

        let current_mtime = find_file_mtime_by_prefix(&dir_path, &prefix).await;
        match (&initial_mtime, &current_mtime) {
            // File appeared
            (None, Some((path, _))) => {
                let bytes = tokio::fs::read(path).await.ok()?;
                let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                return Some(format!("data:{mime};base64,{b64}"));
            }
            // File modified
            (Some((_, old_t)), Some((path, new_t))) if new_t > old_t => {
                let bytes = tokio::fs::read(path).await.ok()?;
                let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                return Some(format!("data:{mime};base64,{b64}"));
            }
            // File deleted
            (Some(_), None) => return None,
            // No change yet
            _ => continue,
        }
    }
}

/// Polls a specific file path. If `require_modification` is false,
/// returns immediately when the file exists. If true, waits for
/// creation or modification. Returns `None` on deletion or timeout.
async fn poll_file(
    path: &std::path::Path,
    timeout: std::time::Duration,
    require_modification: bool,
) -> Option<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    let initial_mtime = file_mtime(path).await;

    // If file exists and we don't require modification, return immediately
    if !require_modification && initial_mtime.is_some() {
        return Some(());
    }

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if tokio::time::Instant::now() >= deadline {
            return None;
        }

        let current_mtime = file_mtime(path).await;
        match (&initial_mtime, &current_mtime) {
            (None, Some(_)) => return Some(()),     // created
            (Some(old), Some(new)) if new > old => return Some(()), // modified
            (Some(_), None) => return None,         // deleted
            _ => continue,                          // no change
        }
    }
}

async fn file_mtime(path: &std::path::Path) -> Option<std::time::SystemTime> {
    tokio::fs::metadata(path).await.ok()?.modified().ok()
}

async fn find_file_mtime_by_prefix(
    dir: &std::path::Path,
    prefix: &str,
) -> Option<(std::path::PathBuf, std::time::SystemTime)> {
    let mut read_dir = tokio::fs::read_dir(dir).await.ok()?;
    while let Some(entry) = read_dir.next_entry().await.ok()? {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with(prefix) {
                let mtime = tokio::fs::metadata(&path).await.ok()?.modified().ok()?;
                return Some((path, mtime));
            }
        }
    }
    None
}

// -----------------------------------------------------------------------
// Subscribe methods — agent completions
// -----------------------------------------------------------------------

pub async fn subscribe_agent_completion(client: &super::super::Client, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "agents/completions", id, timeout, require_modification, jq).await
}

pub async fn subscribe_agent_completion_continuation(client: &super::super::Client, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "agents/completions/continuation", id, timeout, require_modification, jq).await
}

pub async fn subscribe_agent_completion_message(client: &super::super::Client, id: &str, message_index: u64, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "agents/completions/messages", &format!("{id}_{message_index}"), timeout, require_modification, jq).await
}

pub async fn subscribe_agent_completion_message_logprobs(client: &super::super::Client, id: &str, message_index: u64, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "agents/completions/messages/logprobs", &format!("{id}_{message_index}"), timeout, require_modification, jq).await
}

pub async fn subscribe_agent_completion_message_image(client: &super::super::Client, id: &str, message_index: u64, media_index: u64, timeout: std::time::Duration, require_modification: bool) -> Option<String> {
    subscribe_data_url_by_stem(client, "agents/completions/messages/image", &format!("{id}_{message_index}_{media_index}"), timeout, require_modification).await
}

pub async fn subscribe_agent_completion_message_audio(client: &super::super::Client, id: &str, message_index: u64, media_index: u64, timeout: std::time::Duration, require_modification: bool) -> Option<String> {
    subscribe_data_url_by_stem(client, "agents/completions/messages/audio", &format!("{id}_{message_index}_{media_index}"), timeout, require_modification).await
}

pub async fn subscribe_agent_completion_message_video(client: &super::super::Client, id: &str, message_index: u64, media_index: u64, timeout: std::time::Duration, require_modification: bool) -> Option<String> {
    subscribe_data_url_by_stem(client, "agents/completions/messages/video", &format!("{id}_{message_index}_{media_index}"), timeout, require_modification).await
}

pub async fn subscribe_agent_completion_message_file(client: &super::super::Client, id: &str, message_index: u64, media_index: u64, timeout: std::time::Duration, require_modification: bool) -> Option<String> {
    subscribe_data_url_by_stem(client, "agents/completions/messages/file", &format!("{id}_{message_index}_{media_index}"), timeout, require_modification).await
}

// -----------------------------------------------------------------------
// Subscribe methods — vector completions
// -----------------------------------------------------------------------

pub async fn subscribe_vector_completion(client: &super::super::Client, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "vector/completions", id, timeout, require_modification, jq).await
}

// -----------------------------------------------------------------------
// Subscribe methods — function executions
// -----------------------------------------------------------------------

pub async fn subscribe_function_execution(client: &super::super::Client, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "functions/executions", id, timeout, require_modification, jq).await
}

pub async fn subscribe_function_execution_retry_token(client: &super::super::Client, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "functions/executions/retry_token", id, timeout, require_modification, jq).await
}

// -----------------------------------------------------------------------
// Subscribe methods — function inventions
// -----------------------------------------------------------------------

pub async fn subscribe_function_invention(client: &super::super::Client, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "functions/inventions", id, timeout, require_modification, jq).await
}

// -----------------------------------------------------------------------
// Subscribe methods — function inventions recursive
// -----------------------------------------------------------------------

pub async fn subscribe_function_invention_recursive(client: &super::super::Client, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "functions/inventions/recursive", id, timeout, require_modification, jq).await
}

// -----------------------------------------------------------------------
// Subscribe methods — laboratory executions
// -----------------------------------------------------------------------

pub async fn subscribe_laboratory_execution(client: &super::super::Client, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, super::super::Error> {
    subscribe_json(client, "laboratories/executions", id, timeout, require_modification, jq).await
}
