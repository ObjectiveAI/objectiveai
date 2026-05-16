use std::path::PathBuf;

use super::super::{Client, Error};
use super::ListItem;

/// Result of reading a log file — either parsed JSON or a data URL.
#[derive(Debug)]
pub enum LogContent {
    Json(serde_json::Value),
    /// A `data:{mime};base64,{payload}` string.
    DataUrl(String),
}

impl Client {
    fn endpoint_dir(&self, endpoint: &str) -> PathBuf {
        let mut dir = self.logs_dir();
        for segment in endpoint.split('/') {
            dir = dir.join(segment);
        }
        dir
    }

    async fn list_endpoint(
        &self,
        endpoint: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ListItem>, Error> {
        let dir = self.endpoint_dir(endpoint);
        match tokio::fs::metadata(&dir).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(Error::ReadDir(dir, e)),
            Ok(_) => {}
        }
        let mut read_dir = tokio::fs::read_dir(&dir)
            .await
            .map_err(|e| Error::ReadDir(dir.clone(), e))?;
        let mut items = Vec::new();
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| Error::ReadDir(dir.clone(), e))?
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let metadata = tokio::fs::metadata(&path)
                .await
                .map_err(|e| Error::Read(path.clone(), e))?;
            let created = metadata
                .modified()
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

    // -- List methods --------------------------------------------------------

    pub async fn list_agent_completions(&self, offset: usize, limit: usize) -> Result<Vec<ListItem>, Error> {
        self.list_endpoint("agents/completions", offset, limit).await
    }
    pub async fn list_vector_completions(&self, offset: usize, limit: usize) -> Result<Vec<ListItem>, Error> {
        self.list_endpoint("vector/completions", offset, limit).await
    }
    pub async fn list_function_executions(&self, offset: usize, limit: usize) -> Result<Vec<ListItem>, Error> {
        self.list_endpoint("functions/executions", offset, limit).await
    }
    pub async fn list_function_inventions(&self, offset: usize, limit: usize) -> Result<Vec<ListItem>, Error> {
        self.list_endpoint("functions/inventions", offset, limit).await
    }
    pub async fn list_function_inventions_recursive(&self, offset: usize, limit: usize) -> Result<Vec<ListItem>, Error> {
        self.list_endpoint("functions/inventions/recursive", offset, limit).await
    }
    pub async fn list_laboratory_executions(&self, offset: usize, limit: usize) -> Result<Vec<ListItem>, Error> {
        self.list_endpoint("laboratories/executions", offset, limit).await
    }

    // -- Clear helpers + methods --------------------------------------------

    /// Deletes all files (not subdirectories) in the given endpoint directory.
    async fn clear_endpoint(&self, endpoint: &str) -> Result<u64, Error> {
        let dir = self.endpoint_dir(endpoint);
        match tokio::fs::metadata(&dir).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(Error::ReadDir(dir, e)),
            Ok(_) => {}
        }
        let mut read_dir = tokio::fs::read_dir(&dir)
            .await
            .map_err(|e| Error::ReadDir(dir.clone(), e))?;
        let mut count = 0u64;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| Error::ReadDir(dir.clone(), e))?
        {
            let path = entry.path();
            if path.is_file() {
                tokio::fs::remove_file(&path)
                    .await
                    .map_err(|e| Error::Read(path, e))?;
                count += 1;
            }
        }
        Ok(count)
    }

    pub async fn clear_agent_completions(&self) -> Result<u64, Error> {
        self.clear_endpoint("agents/completions").await
    }
    pub async fn clear_agent_completion_continuations(&self) -> Result<u64, Error> {
        self.clear_endpoint("agent/completions/continuation").await
    }
    pub async fn clear_agent_completion_messages(&self) -> Result<u64, Error> {
        self.clear_endpoint("agent/completions/messages").await
    }
    pub async fn clear_agent_completion_message_logprobs(&self) -> Result<u64, Error> {
        self.clear_endpoint("agent/completions/messages/logprobs").await
    }
    pub async fn clear_agent_completion_message_images(&self) -> Result<u64, Error> {
        self.clear_endpoint("agent/completions/messages/image").await
    }
    pub async fn clear_agent_completion_message_audio(&self) -> Result<u64, Error> {
        self.clear_endpoint("agent/completions/messages/audio").await
    }
    pub async fn clear_agent_completion_message_video(&self) -> Result<u64, Error> {
        self.clear_endpoint("agent/completions/messages/video").await
    }
    pub async fn clear_agent_completion_message_files(&self) -> Result<u64, Error> {
        self.clear_endpoint("agent/completions/messages/file").await
    }
    pub async fn clear_vector_completions(&self) -> Result<u64, Error> {
        self.clear_endpoint("vector/completions").await
    }
    pub async fn clear_function_executions(&self) -> Result<u64, Error> {
        self.clear_endpoint("functions/executions").await
    }
    pub async fn clear_function_execution_retry_tokens(&self) -> Result<u64, Error> {
        self.clear_endpoint("functions/executions/retry_token").await
    }
    pub async fn clear_function_inventions(&self) -> Result<u64, Error> {
        self.clear_endpoint("functions/inventions").await
    }
    pub async fn clear_function_inventions_recursive(&self) -> Result<u64, Error> {
        self.clear_endpoint("functions/inventions/recursive").await
    }
    pub async fn clear_laboratory_executions(&self) -> Result<u64, Error> {
        self.clear_endpoint("laboratories/executions").await
    }

    // -- Write methods (LogWriter constructors) -----------------------------

    pub fn write_agent_completion(&self) -> super::LogWriter<crate::agent::completions::response::streaming::AgentCompletionChunk> {
        super::LogWriter::new(self.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
    }
    pub fn write_vector_completion(&self) -> super::LogWriter<crate::vector::completions::response::streaming::VectorCompletionChunk> {
        super::LogWriter::new(self.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
    }
    pub fn write_function_execution(&self) -> super::LogWriter<crate::functions::executions::response::streaming::FunctionExecutionChunk> {
        super::LogWriter::new(self.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
    }
    pub fn write_function_invention(&self) -> super::LogWriter<crate::functions::inventions::response::streaming::FunctionInventionChunk> {
        super::LogWriter::new(self.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
    }
    pub fn write_function_invention_recursive(&self) -> super::LogWriter<crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk> {
        super::LogWriter::new(self.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
    }
    pub fn write_laboratory_execution(&self) -> super::LogWriter<crate::laboratories::executions::response::streaming::LaboratoryExecutionChunk> {
        super::LogWriter::new(self.logs_dir(), |chunk| chunk.produce_files().map(|(_, files)| files))
    }

    // -- Read helpers + methods ---------------------------------------------

    async fn read_json(&self, dir: &str, stem: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        let full = self.logs_dir().join(dir).join(format!("{stem}.json"));
        let bytes = tokio::fs::read(&full)
            .await
            .map_err(|e| Error::Read(full.clone(), e))?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| Error::Parse(full, e))?;
        apply_jq(value, jq)
    }

    /// Finds the first file in `dir` whose name starts with `stem.` (any extension)
    /// and returns it as a data URL.
    async fn read_data_url_by_stem(&self, dir: &str, stem: &str) -> Result<String, Error> {
        use base64::Engine;
        let dir_path = self.logs_dir().join(dir);
        let prefix = format!("{stem}.");
        let mut read_dir = tokio::fs::read_dir(&dir_path)
            .await
            .map_err(|e| Error::ReadDir(dir_path.clone(), e))?;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| Error::ReadDir(dir_path.clone(), e))?
        {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&prefix) {
                    let bytes = tokio::fs::read(&path)
                        .await
                        .map_err(|e| Error::Read(path.clone(), e))?;
                    let mime = mime_guess::from_path(&path)
                        .first_or_octet_stream()
                        .to_string();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    return Ok(format!("data:{mime};base64,{b64}"));
                }
            }
        }
        Err(Error::Read(
            dir_path.join(&prefix),
            std::io::Error::new(std::io::ErrorKind::NotFound, "no matching file"),
        ))
    }

    pub async fn read_agent_completion(&self, id: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("agents/completions", id, jq).await
    }
    pub async fn read_agent_completion_continuation(&self, id: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("agents/completions/continuation", id, jq).await
    }
    pub async fn read_agent_completion_message(&self, id: &str, message_index: u64, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("agents/completions/messages", &format!("{id}_{message_index}"), jq).await
    }
    pub async fn read_agent_completion_message_logprobs(&self, id: &str, message_index: u64, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("agents/completions/messages/logprobs", &format!("{id}_{message_index}"), jq).await
    }
    pub async fn read_agent_completion_message_image(&self, id: &str, message_index: u64, media_index: u64) -> Result<String, Error> {
        self.read_data_url_by_stem("agents/completions/messages/image", &format!("{id}_{message_index}_{media_index}")).await
    }
    pub async fn read_agent_completion_message_audio(&self, id: &str, message_index: u64, media_index: u64) -> Result<String, Error> {
        self.read_data_url_by_stem("agents/completions/messages/audio", &format!("{id}_{message_index}_{media_index}")).await
    }
    pub async fn read_agent_completion_message_video(&self, id: &str, message_index: u64, media_index: u64) -> Result<String, Error> {
        self.read_data_url_by_stem("agents/completions/messages/video", &format!("{id}_{message_index}_{media_index}")).await
    }
    pub async fn read_agent_completion_message_file(&self, id: &str, message_index: u64, media_index: u64) -> Result<String, Error> {
        self.read_data_url_by_stem("agents/completions/messages/file", &format!("{id}_{message_index}_{media_index}")).await
    }
    pub async fn read_vector_completion(&self, id: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("vector/completions", id, jq).await
    }
    pub async fn read_function_execution(&self, id: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("functions/executions", id, jq).await
    }
    pub async fn read_function_execution_retry_token(&self, id: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("functions/executions/retry_token", id, jq).await
    }
    pub async fn read_function_invention(&self, id: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("functions/inventions", id, jq).await
    }
    pub async fn read_function_invention_recursive(&self, id: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("functions/inventions/recursive", id, jq).await
    }
    pub async fn read_laboratory_execution(&self, id: &str, jq: Option<&str>) -> Result<serde_json::Value, Error> {
        self.read_json("laboratories/executions", id, jq).await
    }

    // -- Subscribe helpers + methods ----------------------------------------

    /// Polls for a JSON file. If `require_modification` is false, returns
    /// immediately when the file exists. If true, waits for creation or
    /// modification. Returns `Ok(None)` on deletion or timeout. When `jq` is
    /// provided, the result is run through the filter before returning.
    async fn subscribe_json(
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
        jq: Option<&str>,
    ) -> Result<Option<serde_json::Value>, Error> {
        let full = self.logs_dir().join(dir).join(format!("{stem}.json"));
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
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Option<String> {
        use base64::Engine;
        let dir_path = self.logs_dir().join(dir);
        let prefix = format!("{stem}.");

        let deadline = tokio::time::Instant::now() + timeout;
        let initial_mtime = find_file_mtime_by_prefix(&dir_path, &prefix).await;

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
                (None, Some((path, _))) => {
                    let bytes = tokio::fs::read(path).await.ok()?;
                    let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    return Some(format!("data:{mime};base64,{b64}"));
                }
                (Some((_, old_t)), Some((path, new_t))) if new_t > old_t => {
                    let bytes = tokio::fs::read(path).await.ok()?;
                    let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    return Some(format!("data:{mime};base64,{b64}"));
                }
                (Some(_), None) => return None,
                _ => continue,
            }
        }
    }

    pub async fn subscribe_agent_completion(&self, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("agents/completions", id, timeout, require_modification, jq).await
    }
    pub async fn subscribe_agent_completion_continuation(&self, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("agents/completions/continuation", id, timeout, require_modification, jq).await
    }
    pub async fn subscribe_agent_completion_message(&self, id: &str, message_index: u64, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("agents/completions/messages", &format!("{id}_{message_index}"), timeout, require_modification, jq).await
    }
    pub async fn subscribe_agent_completion_message_logprobs(&self, id: &str, message_index: u64, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("agents/completions/messages/logprobs", &format!("{id}_{message_index}"), timeout, require_modification, jq).await
    }
    pub async fn subscribe_agent_completion_message_image(&self, id: &str, message_index: u64, media_index: u64, timeout: std::time::Duration, require_modification: bool) -> Option<String> {
        self.subscribe_data_url_by_stem("agents/completions/messages/image", &format!("{id}_{message_index}_{media_index}"), timeout, require_modification).await
    }
    pub async fn subscribe_agent_completion_message_audio(&self, id: &str, message_index: u64, media_index: u64, timeout: std::time::Duration, require_modification: bool) -> Option<String> {
        self.subscribe_data_url_by_stem("agents/completions/messages/audio", &format!("{id}_{message_index}_{media_index}"), timeout, require_modification).await
    }
    pub async fn subscribe_agent_completion_message_video(&self, id: &str, message_index: u64, media_index: u64, timeout: std::time::Duration, require_modification: bool) -> Option<String> {
        self.subscribe_data_url_by_stem("agents/completions/messages/video", &format!("{id}_{message_index}_{media_index}"), timeout, require_modification).await
    }
    pub async fn subscribe_agent_completion_message_file(&self, id: &str, message_index: u64, media_index: u64, timeout: std::time::Duration, require_modification: bool) -> Option<String> {
        self.subscribe_data_url_by_stem("agents/completions/messages/file", &format!("{id}_{message_index}_{media_index}"), timeout, require_modification).await
    }
    pub async fn subscribe_vector_completion(&self, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("vector/completions", id, timeout, require_modification, jq).await
    }
    pub async fn subscribe_function_execution(&self, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("functions/executions", id, timeout, require_modification, jq).await
    }
    pub async fn subscribe_function_execution_retry_token(&self, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("functions/executions/retry_token", id, timeout, require_modification, jq).await
    }
    pub async fn subscribe_function_invention(&self, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("functions/inventions", id, timeout, require_modification, jq).await
    }
    pub async fn subscribe_function_invention_recursive(&self, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("functions/inventions/recursive", id, timeout, require_modification, jq).await
    }
    pub async fn subscribe_laboratory_execution(&self, id: &str, timeout: std::time::Duration, require_modification: bool, jq: Option<&str>) -> Result<Option<serde_json::Value>, Error> {
        self.subscribe_json("laboratories/executions", id, timeout, require_modification, jq).await
    }
}

// -- Pure helpers (no &Client) ---------------------------------------------

/// Applies a jq filter to a JSON value, collapsing the multi-result vector
/// the same way the CLI `config get` command does: a single result is
/// unwrapped, an empty result becomes JSON null, and multiple results are
/// wrapped as an array. When `jq` is `None`, the value is returned as-is.
fn apply_jq(value: serde_json::Value, jq: Option<&str>) -> Result<serde_json::Value, Error> {
    let Some(filter) = jq else { return Ok(value); };
    let mut results = super::super::run_jq(&value, filter)?;
    Ok(match results.len() {
        0 => serde_json::Value::Null,
        1 => results.remove(0),
        _ => serde_json::Value::Array(results),
    })
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
            (None, Some(_)) => return Some(()),
            (Some(old), Some(new)) if new > old => return Some(()),
            (Some(_), None) => return None,
            _ => continue,
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
