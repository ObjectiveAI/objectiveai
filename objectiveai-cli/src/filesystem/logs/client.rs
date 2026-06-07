use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::{Client, Error};
use super::ListItem;
use objectiveai_sdk::agent::completions::message::{
    File, ImageUrl, InputAudio, VideoUrl,
};

/// Result of reading a log file. The variant is determined by **which
/// typed `read_*` method the caller invoked** (or, for
/// [`Client::read_file_by_id`], by the on-disk folder the path
/// classified into). Nothing is guessed from the bytes — each
/// variant is picked at the call site that already knows the kind.
///
/// Wire form (when wrapped in
/// `TypedNotificationValue::LogContent`): the outer `type:
/// "log_content"` envelope from `TypedNotificationValue` plus this
/// inner `kind` discriminator (`type`/`kind` swap relative to the
/// historical shape — needed to avoid a key collision when the
/// outer enum flattens through `Notification`):
///
/// ```text
/// {"type":"log_content","kind":"json", "content":{...}}
/// {"type":"log_content","kind":"text", "text":"..."}
/// {"type":"log_content","kind":"image","image_url":{"url":"data:image/png;base64,..."}}
/// {"type":"log_content","kind":"audio","input_audio":{"data":"<base64>","format":"audio/mpeg"}}
/// {"type":"log_content","kind":"video","video_url":{"url":"data:video/mp4;base64,..."}}
/// {"type":"log_content","kind":"file", "file":{"file_data":"<base64>","filename":"<name>"}}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[schemars(rename = "filesystem.logs.LogContent")]
pub enum LogContent {
    /// A `.json` envelope parsed as a structured value.
    #[schemars(title = "Json")]
    Json { content: serde_json::Value },
    /// A `.txt` file content.
    #[schemars(title = "Text")]
    Text { text: String },
    /// An image media file under `messages/image/`, etc.
    #[schemars(title = "Image")]
    Image { image_url: ImageUrl },
    /// An audio media file under `messages/audio/`, etc.
    #[schemars(title = "Audio")]
    Audio { input_audio: InputAudio },
    /// A video media file under `messages/video/`, etc.
    #[schemars(title = "Video")]
    Video { video_url: VideoUrl },
    /// A generic file under `messages/file/`, etc.
    #[schemars(title = "File")]
    File { file: File },
}

impl LogContent {
    /// `LogContent::json(value)` — constructor sugar for the `Json`
    /// variant. Useful at `.map(LogContent::json)` call sites that
    /// were `.map(LogContent::Json)` before the struct-variant rename.
    pub fn json(content: serde_json::Value) -> Self {
        Self::Json { content }
    }
    /// Constructor sugar for the `Text` variant.
    pub fn text(text: String) -> Self {
        Self::Text { text }
    }
    /// Constructor sugar for the `Image` variant.
    pub fn image(image_url: ImageUrl) -> Self {
        Self::Image { image_url }
    }
    /// Constructor sugar for the `Audio` variant.
    pub fn audio(input_audio: InputAudio) -> Self {
        Self::Audio { input_audio }
    }
    /// Constructor sugar for the `Video` variant.
    pub fn video(video_url: VideoUrl) -> Self {
        Self::Video { video_url }
    }
    /// Constructor sugar for the `File` variant.
    pub fn file(file: File) -> Self {
        Self::File { file }
    }
}

/// Lossless projection into the agent-side rich-content surface. The
/// MCP formatter funnels every cli output through `RichContentPart`
/// before handing off to `ContentBlock`; this `From` is the entry
/// point for the `LogContent` family.
///
/// `LogContent::Json` projects to a `Text` part carrying the
/// JSON-encoded body (closest `RichContentPart` representation —
/// there's no structured-JSON variant). Every other LogContent
/// variant maps to the matching typed `RichContentPart`.
impl From<LogContent> for objectiveai_sdk::agent::completions::message::RichContentPart {
    fn from(log: LogContent) -> Self {
        use objectiveai_sdk::agent::completions::message::RichContentPart;
        match log {
            LogContent::Json { content } => RichContentPart::Text {
                text: serde_json::to_string(&content).unwrap_or_default(),
            },
            LogContent::Text { text } => RichContentPart::Text { text },
            LogContent::Image { image_url } => {
                RichContentPart::ImageUrl { image_url }
            }
            LogContent::Audio { input_audio } => {
                RichContentPart::InputAudio { input_audio }
            }
            LogContent::Video { video_url } => {
                RichContentPart::InputVideo { video_url }
            }
            LogContent::File { file } => RichContentPart::File { file },
        }
    }
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
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
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
                Some(s) => s,
                None => continue,
            };
            let id = stem.to_string();
            let metadata = tokio::fs::metadata(&path)
                .await
                .map_err(|e| Error::Read(path.clone(), e))?;
            let created = metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            items.push(ListItem { id, created });
        }
        items.sort_by(|a, b| b.created.cmp(&a.created));
        if offset > 0 || limit < usize::MAX {
            items = items.into_iter().skip(offset).take(limit).collect();
        }
        Ok(items)
    }

    // -- List methods --------------------------------------------------------

    pub async fn list_agent_completions(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<
        Vec<objectiveai_sdk::cli::command::logs::agents::completions::response::list::ResponseItem>,
        Error,
    > {
        let items = self
            .list_endpoint("agents/completions/response", offset, limit)
            .await?;
        Ok(items
            .into_iter()
            .map(|i| {
                objectiveai_sdk::cli::command::logs::agents::completions::response::list::ResponseItem {
                    id: i.id,
                    created: i.created,
                }
            })
            .collect())
    }
    pub async fn list_vector_completions(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<
        Vec<objectiveai_sdk::cli::command::logs::vector::completions::response::list::ResponseItem>,
        Error,
    > {
        let items = self
            .list_endpoint("vector/completions/response", offset, limit)
            .await?;
        Ok(items
            .into_iter()
            .map(|i| {
                objectiveai_sdk::cli::command::logs::vector::completions::response::list::ResponseItem {
                    id: i.id,
                    created: i.created,
                }
            })
            .collect())
    }
    pub async fn list_function_executions(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<
        Vec<objectiveai_sdk::cli::command::logs::functions::executions::response::list::ResponseItem>,
        Error,
    > {
        let items = self
            .list_endpoint("functions/executions/response", offset, limit)
            .await?;
        Ok(items
            .into_iter()
            .map(|i| {
                objectiveai_sdk::cli::command::logs::functions::executions::response::list::ResponseItem {
                    id: i.id,
                    created: i.created,
                }
            })
            .collect())
    }
    pub async fn list_function_inventions(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<
        Vec<objectiveai_sdk::cli::command::logs::functions::inventions::response::list::ResponseItem>,
        Error,
    > {
        let items = self
            .list_endpoint("functions/inventions/response", offset, limit)
            .await?;
        Ok(items
            .into_iter()
            .map(|i| {
                objectiveai_sdk::cli::command::logs::functions::inventions::response::list::ResponseItem {
                    id: i.id,
                    created: i.created,
                }
            })
            .collect())
    }
    pub async fn list_function_inventions_recursive(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<
        Vec<objectiveai_sdk::cli::command::logs::functions::inventions::recursive::response::list::ResponseItem>,
        Error,
    > {
        let items = self
            .list_endpoint("functions/inventions/recursive/response", offset, limit)
            .await?;
        Ok(items
            .into_iter()
            .map(|i| {
                objectiveai_sdk::cli::command::logs::functions::inventions::recursive::response::list::ResponseItem {
                    id: i.id,
                    created: i.created,
                }
            })
            .collect())
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
        self.clear_endpoint("agents/completions/response").await
    }
    pub async fn clear_agent_completion_continuations(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint("agents/completions/response/continuation")
            .await
    }
    pub async fn clear_agent_completion_messages_assistant(
        &self,
    ) -> Result<u64, Error> {
        // `clear_endpoint` skips the nested per-kind subdirs
        // (logprobs, text, ...), which have their own clear methods.
        self.clear_endpoint("agents/completions/response/messages/assistant")
            .await
    }
    pub async fn clear_agent_completion_messages_tool(
        &self,
    ) -> Result<u64, Error> {
        // `clear_endpoint` skips the nested per-media subdirs
        // (text, image, ...).
        self.clear_endpoint("agents/completions/response/messages/tool")
            .await
    }
    pub async fn clear_agent_completion_message_assistant_logprobs(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint(
            "agents/completions/response/messages/assistant/logprobs",
        )
        .await
    }
    pub async fn clear_agent_completion_message_assistant_reasoning(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint(
            "agents/completions/response/messages/assistant/reasoning",
        )
        .await
    }
    pub async fn clear_agent_completion_message_assistant_refusal(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint(
            "agents/completions/response/messages/assistant/refusal",
        )
        .await
    }
    pub async fn clear_agent_completion_message_assistant_tool_calls(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint(
            "agents/completions/response/messages/assistant/tool_calls",
        )
        .await
    }
    pub async fn clear_agent_completion_message_assistant_images(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint(
            "agents/completions/response/messages/assistant/image",
        )
        .await
    }
    pub async fn clear_agent_completion_message_assistant_audio(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint(
            "agents/completions/response/messages/assistant/audio",
        )
        .await
    }
    pub async fn clear_agent_completion_message_assistant_video(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint(
            "agents/completions/response/messages/assistant/video",
        )
        .await
    }
    pub async fn clear_agent_completion_message_assistant_files(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint(
            "agents/completions/response/messages/assistant/file",
        )
        .await
    }
    pub async fn clear_vector_completions(&self) -> Result<u64, Error> {
        self.clear_endpoint("vector/completions/response").await
    }
    pub async fn clear_function_executions(&self) -> Result<u64, Error> {
        self.clear_endpoint("functions/executions/response").await
    }
    pub async fn clear_function_execution_retry_tokens(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint("functions/executions/response/retry_token")
            .await
    }
    pub async fn clear_function_inventions(&self) -> Result<u64, Error> {
        self.clear_endpoint("functions/inventions/response").await
    }
    pub async fn clear_function_inventions_recursive(
        &self,
    ) -> Result<u64, Error> {
        self.clear_endpoint("functions/inventions/recursive/response")
            .await
    }

    // -- Write methods (LogWriter constructors) -----------------------------

    pub fn write_agent_completion(
        &self,
        db: &crate::db::Pool,
        request: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
    ) -> Result<super::LogWriter<objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk>, crate::error::Error>{
        use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
        let queue =
            crate::db::messages::Queue::new(db.clone(), self.logs_dir());
        Ok(super::LogWriter::new(
            self.logs_dir(),
            |chunk: &AgentCompletionChunk| {
                crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_files(chunk)
                    .map(|(_, files)| files)
            },
        )
        .with_request("agents/completions/request", request)?
        .with_queue(
            queue,
            Some(objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind::AgentCompletionRequest),
            |chunk: &AgentCompletionChunk| {
                Box::new(crate::logs::agents::completions::response::streaming::agent_completion_chunk::produce_message_rows(chunk))
            },
        ))
    }
    pub fn write_vector_completion(
        &self,
        db: &crate::db::Pool,
        request: &objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams,
    ) -> Result<super::LogWriter<objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk>, crate::error::Error>{
        use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;
        let queue =
            crate::db::messages::Queue::new(db.clone(), self.logs_dir());
        Ok(super::LogWriter::new(
            self.logs_dir(),
            |chunk: &VectorCompletionChunk| {
                crate::logs::vector::completions::response::streaming::vector_completion_chunk::produce_files(chunk)
                    .map(|(_, files)| files)
            },
        )
        .with_request("vector/completions/request", request)?
        .with_queue(queue, None, |chunk: &VectorCompletionChunk| {
            Box::new(crate::logs::vector::completions::response::streaming::vector_completion_chunk::produce_message_rows(chunk))
        }))
    }
    pub fn write_function_execution(
        &self,
        db: &crate::db::Pool,
        request: &objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams,
    ) -> Result<super::LogWriter<objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk>, crate::error::Error>{
        use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
        let queue =
            crate::db::messages::Queue::new(db.clone(), self.logs_dir());
        Ok(super::LogWriter::new(
            self.logs_dir(),
            |chunk: &FunctionExecutionChunk| {
                crate::logs::functions::executions::response::streaming::function_execution_chunk::produce_files(chunk)
                    .map(|(_, files)| files)
            },
        )
        .with_request("functions/executions/request", request)?
        .with_queue(
            queue,
            Some(
                objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind::FunctionExecutionRequest,
            ),
            |chunk: &FunctionExecutionChunk| crate::logs::functions::executions::response::streaming::function_execution_chunk::produce_message_rows(chunk),
        ))
    }
    pub fn write_function_invention(
        &self,
        db: &crate::db::Pool,
        request: &objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams,
    ) -> Result<super::LogWriter<objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk>, crate::error::Error>{
        use objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk;
        let queue =
            crate::db::messages::Queue::new(db.clone(), self.logs_dir());
        Ok(super::LogWriter::new(
            self.logs_dir(),
            |chunk: &FunctionInventionChunk| {
                crate::logs::functions::inventions::response::streaming::function_invention_chunk::produce_files(chunk)
                    .map(|(_, files)| files)
            },
        )
        .with_request("functions/inventions/request", request)?
        .with_queue(queue, None, |chunk: &FunctionInventionChunk| {
            Box::new(crate::logs::functions::inventions::response::streaming::function_invention_chunk::produce_message_rows(chunk))
        }))
    }
    pub fn write_function_invention_recursive(
        &self,
        db: &crate::db::Pool,
        request: &objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams,
    ) -> Result<super::LogWriter<objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk>, crate::error::Error>{
        use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk;
        let queue =
            crate::db::messages::Queue::new(db.clone(), self.logs_dir());
        Ok(super::LogWriter::new(self.logs_dir(), |chunk: &FunctionInventionRecursiveChunk| crate::logs::functions::inventions::recursive::response::streaming::function_invention_recursive_chunk::produce_files(chunk).map(|(_, files)| files))
            .with_request("functions/inventions/recursive/request", request)?
            .with_queue(
                queue,
                Some(objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind::FunctionInventionRecursiveRequest),
                |chunk: &FunctionInventionRecursiveChunk| Box::new(crate::logs::functions::inventions::recursive::response::streaming::function_invention_recursive_chunk::produce_message_rows(chunk)),
            ))
    }

    // -- Read helpers + methods ---------------------------------------------

    async fn read_json(
        &self,
        dir: &str,
        stem: &str,
        jq: Option<&str>,
    ) -> Result<serde_json::Value, Error> {
        let full = self.logs_dir().join(dir).join(format!("{stem}.json"));
        let bytes = tokio::fs::read(&full)
            .await
            .map_err(|e| Error::Read(full.clone(), e))?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| Error::Parse(full, e))?;
        apply_jq(value, jq)
    }

    /// Reads `<dir>/<stem>.json` and deserializes its contents into the
    /// caller-specified `T`. Sits on top of [`Self::read_json`] (no jq
    /// — the typed reads pass `None`) and surfaces a typed-shape
    /// deserialization failure as [`Error::TypedDeserialize`] with the
    /// originating file path attached. Used by the typed `read_*`
    /// methods that match the SDK's per-leaf `Response` shapes.
    async fn read_json_typed<T: serde::de::DeserializeOwned>(
        &self,
        dir: &str,
        stem: &str,
    ) -> Result<T, Error> {
        let value = self.read_json(dir, stem, None).await?;
        let full = self.logs_dir().join(dir).join(format!("{stem}.json"));
        serde_json::from_value(value).map_err(|e| Error::TypedDeserialize(full, e))
    }

    /// Reads a `.txt` file at `<dir>/<stem>.txt` as a UTF-8 string.
    /// Sibling to [`Self::read_json`] / [`Self::read_data_url_by_stem`]
    /// for the text-content writers (`extract_media` â†’ `<...>/text/<stem>.txt`).
    async fn read_text(&self, dir: &str, stem: &str) -> Result<String, Error> {
        let full = self.logs_dir().join(dir).join(format!("{stem}.txt"));
        let bytes = tokio::fs::read(&full)
            .await
            .map_err(|e| Error::Read(full.clone(), e))?;
        String::from_utf8(bytes).map_err(|e| Error::Utf8(full, e))
    }

    /// Finds the first file in `dir` whose name starts with `stem.`
    /// (any extension), reads its bytes, and returns
    /// `(mime, base64_payload, filename)`. The mime is derived from
    /// the extension via `mime_guess`; the filename is the bare
    /// `<stem>.<ext>` from disk (useful for the `File` variant of
    /// [`LogContent`]).
    async fn read_media_parts_by_stem(
        &self,
        dir: &str,
        stem: &str,
    ) -> Result<(String, String, Option<String>), Error> {
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
                    let b64 = base64::engine::general_purpose::STANDARD
                        .encode(&bytes);
                    let filename = Some(name.to_string());
                    return Ok((mime, b64, filename));
                }
            }
        }
        Err(Error::Read(
            dir_path.join(&prefix),
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no matching file",
            ),
        ))
    }

    /// Read the media file at `dir/<stem>.*` and assemble an
    /// [`ImageUrl`] with the `data:<mime>;base64,<payload>` URL form.
    /// Called by every `read_*_image` method.
    async fn read_image_by_stem(
        &self,
        dir: &str,
        stem: &str,
    ) -> Result<ImageUrl, Error> {
        let (mime, b64, _) = self.read_media_parts_by_stem(dir, stem).await?;
        Ok(ImageUrl {
            url: format!("data:{mime};base64,{b64}"),
            detail: None,
        })
    }

    /// Read the media file at `dir/<stem>.*` and assemble an
    /// [`InputAudio`] (raw base64 + format string).
    /// Called by every `read_*_audio` method.
    async fn read_audio_by_stem(
        &self,
        dir: &str,
        stem: &str,
    ) -> Result<InputAudio, Error> {
        let (mime, b64, _) = self.read_media_parts_by_stem(dir, stem).await?;
        Ok(InputAudio {
            data: b64,
            format: mime,
        })
    }

    /// Read the media file at `dir/<stem>.*` and assemble a
    /// [`VideoUrl`] with the `data:<mime>;base64,<payload>` URL form.
    /// Called by every `read_*_video` method.
    async fn read_video_by_stem(
        &self,
        dir: &str,
        stem: &str,
    ) -> Result<VideoUrl, Error> {
        let (mime, b64, _) = self.read_media_parts_by_stem(dir, stem).await?;
        Ok(VideoUrl {
            url: format!("data:{mime};base64,{b64}"),
        })
    }

    /// Read the file at `dir/<stem>.*` and assemble a [`File`]
    /// carrying the raw base64 payload + on-disk filename. Called by
    /// every `read_*_file` method.
    async fn read_file_by_stem(
        &self,
        dir: &str,
        stem: &str,
    ) -> Result<File, Error> {
        let (_, b64, filename) =
            self.read_media_parts_by_stem(dir, stem).await?;
        Ok(File {
            file_data: Some(b64),
            filename,
            file_id: None,
            file_url: None,
        })
    }

    pub async fn read_agent_completion(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunkLog,
        Error,
    > {
        self.read_json_typed("agents/completions/response", id).await
    }
    pub async fn read_agent_completion_request(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::agent::completions::request::AgentCompletionCreateParamsLog,
        Error,
    > {
        self.read_json_typed("agents/completions/request", id).await
    }
    pub async fn read_agent_completion_continuation(
        &self,
        id: &str,
    ) -> Result<String, Error> {
        self.read_text("agents/completions/response/continuation", id)
            .await
    }
    /// The assistant envelope, exactly as written —
    /// an [`AssistantResponseChunkLog`], not the lossy `MessageLog`
    /// projection.
    ///
    /// [`AssistantResponseChunkLog`]: objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunkLog
    pub async fn read_agent_completion_message_assistant(
        &self,
        id: &str,
        message_index: u64,
    ) -> Result<
        objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunkLog,
        Error,
    > {
        self.read_json_typed(
            "agents/completions/response/messages/assistant",
            &format!("{id}_{message_index}"),
        )
        .await
    }
    /// The tool envelope, exactly as written — a [`ToolResponseLog`].
    ///
    /// [`ToolResponseLog`]: objectiveai_sdk::agent::completions::response::ToolResponseLog
    pub async fn read_agent_completion_message_tool(
        &self,
        id: &str,
        message_index: u64,
    ) -> Result<
        objectiveai_sdk::agent::completions::response::ToolResponseLog,
        Error,
    > {
        self.read_json_typed(
            "agents/completions/response/messages/tool",
            &format!("{id}_{message_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_assistant_logprobs(
        &self,
        id: &str,
        message_index: u64,
    ) -> Result<objectiveai_sdk::agent::completions::response::Logprobs, Error> {
        self.read_json_typed(
            "agents/completions/response/messages/assistant/logprobs",
            &format!("{id}_{message_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_assistant_reasoning(
        &self,
        id: &str,
        message_index: u64,
    ) -> Result<String, Error> {
        // Raw text — `.txt` on disk, not `.json`. (Writer:
        // `assistant_response_chunk.rs::extract_*` puts reasoning
        // there as plain bytes; no JSON quoting.)
        self.read_text(
            "agents/completions/response/messages/assistant/reasoning",
            &format!("{id}_{message_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_assistant_refusal(
        &self,
        id: &str,
        message_index: u64,
    ) -> Result<String, Error> {
        // Raw text — `.txt` on disk, not `.json`. See reasoning.
        self.read_text(
            "agents/completions/response/messages/assistant/refusal",
            &format!("{id}_{message_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_assistant_tool_call(
        &self,
        id: &str,
        message_index: u64,
        tool_call_index: u64,
    ) -> Result<objectiveai_sdk::agent::completions::message::AssistantToolCallDelta, Error> {
        self.read_json_typed(
            "agents/completions/response/messages/assistant/tool_calls",
            &format!("{id}_{message_index}_{tool_call_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_assistant_image(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<ImageUrl, Error> {
        self.read_image_by_stem(
            "agents/completions/response/messages/assistant/image",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_assistant_audio(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<InputAudio, Error> {
        self.read_audio_by_stem(
            "agents/completions/response/messages/assistant/audio",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_assistant_video(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<VideoUrl, Error> {
        self.read_video_by_stem(
            "agents/completions/response/messages/assistant/video",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_assistant_file(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<File, Error> {
        self.read_file_by_stem(
            "agents/completions/response/messages/assistant/file",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_vector_completion(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunkLog,
        Error,
    > {
        self.read_json_typed("vector/completions/response", id).await
    }
    pub async fn read_vector_completion_request(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams,
        Error,
    > {
        self.read_json_typed("vector/completions/request", id).await
    }
    pub async fn read_function_execution(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunkLog,
        Error,
    > {
        self.read_json_typed("functions/executions/response", id)
            .await
    }
    pub async fn read_function_execution_request(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParamsLog,
        Error,
    > {
        self.read_json_typed("functions/executions/request", id)
            .await
    }
    pub async fn read_function_execution_retry_token(
        &self,
        id: &str,
    ) -> Result<String, Error> {
        self.read_text("functions/executions/response/retry_token", id)
            .await
    }
    pub async fn read_function_invention(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunkLog,
        Error,
    > {
        self.read_json_typed("functions/inventions/response", id)
            .await
    }
    pub async fn read_function_invention_request(
        &self,
        id: &str,
    ) -> Result<objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams, Error>
    {
        self.read_json_typed("functions/inventions/request", id)
            .await
    }
    pub async fn read_function_invention_recursive(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunkLog,
        Error,
    > {
        self.read_json_typed("functions/inventions/recursive/response", id)
            .await
    }
    pub async fn read_function_invention_recursive_request(
        &self,
        id: &str,
    ) -> Result<
        objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParamsLog,
        Error,
    > {
        self.read_json_typed("functions/inventions/recursive/request", id)
            .await
    }

    // -- Assistant message content (response side) -------------------------
    //
    // `RichContent::Text(_)` â†’ `<id>_<message_index>.txt` (no media_index).
    // `RichContent::Parts([... Text { text } ...])` â†’ one file per part,
    // `<id>_<message_index>_<part>.<ext>`. The text reader takes
    // `media_index: Option<u64>` to cover both cases.

    pub async fn read_agent_completion_message_assistant_text(
        &self,
        id: &str,
        message_index: u64,
        media_index: Option<u64>,
    ) -> Result<String, Error> {
        let stem = text_stem(id, message_index, media_index);
        self.read_text(
            "agents/completions/response/messages/assistant/text",
            &stem,
        )
        .await
    }

    // -- Tool response content (response side, under .../messages/tool/) ---

    pub async fn read_agent_completion_message_tool_text(
        &self,
        id: &str,
        message_index: u64,
        media_index: Option<u64>,
    ) -> Result<String, Error> {
        let stem = text_stem(id, message_index, media_index);
        self.read_text("agents/completions/response/messages/tool/text", &stem)
            .await
    }
    pub async fn read_agent_completion_message_tool_image(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<ImageUrl, Error> {
        self.read_image_by_stem(
            "agents/completions/response/messages/tool/image",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_tool_audio(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<InputAudio, Error> {
        self.read_audio_by_stem(
            "agents/completions/response/messages/tool/audio",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_tool_video(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<VideoUrl, Error> {
        self.read_video_by_stem(
            "agents/completions/response/messages/tool/video",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_message_tool_file(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<File, Error> {
        self.read_file_by_stem(
            "agents/completions/response/messages/tool/file",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }

    // -- Request-side message content --------------------------------------

    pub async fn read_agent_completion_request_message_text(
        &self,
        id: &str,
        message_index: u64,
        media_index: Option<u64>,
    ) -> Result<String, Error> {
        let stem = text_stem(id, message_index, media_index);
        self.read_text("agents/completions/request/messages/text", &stem)
            .await
    }
    pub async fn read_agent_completion_request_message_image(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<ImageUrl, Error> {
        self.read_image_by_stem(
            "agents/completions/request/messages/image",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_request_message_audio(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<InputAudio, Error> {
        self.read_audio_by_stem(
            "agents/completions/request/messages/audio",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_request_message_video(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<VideoUrl, Error> {
        self.read_video_by_stem(
            "agents/completions/request/messages/video",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_request_message_file(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
    ) -> Result<File, Error> {
        self.read_file_by_stem(
            "agents/completions/request/messages/file",
            &format!("{id}_{message_index}_{media_index}"),
        )
        .await
    }
    /// The per-role `MessageLog` envelope at
    /// `request/messages/<id>_<msg>.json`. Envelopes for every role
    /// share the bare `messages/` namespace (request message indices
    /// are unique across roles); only the assistant-only extras
    /// (reasoning / refusal / tool_calls) live under
    /// `messages/assistant/`.
    pub async fn read_agent_completion_request_message(
        &self,
        id: &str,
        message_index: u64,
    ) -> Result<objectiveai_sdk::agent::completions::message::MessageLog, Error> {
        self.read_json_typed(
            "agents/completions/request/messages",
            &format!("{id}_{message_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_request_message_assistant_reasoning(
        &self,
        id: &str,
        message_index: u64,
    ) -> Result<String, Error> {
        // A JSON-encoded string — the request-side writer
        // (`assistant_message.rs::extract`) serializes reasoning with
        // `serde_json::to_vec_pretty`, unlike the response side's
        // raw-bytes `.txt`.
        self.read_json_typed(
            "agents/completions/request/messages/assistant/reasoning",
            &format!("{id}_{message_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_request_message_assistant_refusal(
        &self,
        id: &str,
        message_index: u64,
    ) -> Result<String, Error> {
        // A JSON-encoded string — see reasoning.
        self.read_json_typed(
            "agents/completions/request/messages/assistant/refusal",
            &format!("{id}_{message_index}"),
        )
        .await
    }
    /// Request-side tool calls are full `AssistantToolCall`s (the
    /// caller supplied them whole), unlike the response side's
    /// streaming deltas. Returned exactly as written — no delta
    /// conversion.
    pub async fn read_agent_completion_request_message_assistant_tool_call(
        &self,
        id: &str,
        message_index: u64,
        tool_call_index: u64,
    ) -> Result<objectiveai_sdk::agent::completions::message::AssistantToolCall, Error> {
        self.read_json_typed(
            "agents/completions/request/messages/assistant/tool_calls",
            &format!("{id}_{message_index}_{tool_call_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_request_continuation(
        &self,
        id: &str,
    ) -> Result<String, Error> {
        self.read_text("agents/completions/request/continuation", id)
            .await
    }

    // -- Notification content ----------------------------------------------

    pub async fn read_agent_completion_notification_text(
        &self,
        response_id: &str,
        index: u64,
        media_index: Option<u64>,
    ) -> Result<String, Error> {
        let stem = text_stem(response_id, index, media_index);
        self.read_text("agents/completions/request/notifications/text", &stem)
            .await
    }
    pub async fn read_agent_completion_notification_image(
        &self,
        response_id: &str,
        index: u64,
        media_index: u64,
    ) -> Result<ImageUrl, Error> {
        self.read_image_by_stem(
            "agents/completions/request/notifications/image",
            &format!("{response_id}_{index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_notification_audio(
        &self,
        response_id: &str,
        index: u64,
        media_index: u64,
    ) -> Result<InputAudio, Error> {
        self.read_audio_by_stem(
            "agents/completions/request/notifications/audio",
            &format!("{response_id}_{index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_notification_video(
        &self,
        response_id: &str,
        index: u64,
        media_index: u64,
    ) -> Result<VideoUrl, Error> {
        self.read_video_by_stem(
            "agents/completions/request/notifications/video",
            &format!("{response_id}_{index}_{media_index}"),
        )
        .await
    }
    pub async fn read_agent_completion_notification_file(
        &self,
        response_id: &str,
        index: u64,
        media_index: u64,
    ) -> Result<File, Error> {
        self.read_file_by_stem(
            "agents/completions/request/notifications/file",
            &format!("{response_id}_{index}_{media_index}"),
        )
        .await
    }

    // -- Subscribe helpers + methods ----------------------------------------

    /// Polls for a `.txt` file. Sibling to [`Self::subscribe_json`] for
    /// the raw-string writers (response-side continuation, retry_token).
    /// Returns `Ok(None)` on deletion or timeout.
    async fn subscribe_text(
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<Option<String>, Error> {
        let full = self.logs_dir().join(dir).join(format!("{stem}.txt"));
        if poll_file(&full, timeout, require_modification)
            .await
            .is_none()
        {
            return Ok(None);
        }
        let bytes = match tokio::fs::read(&full).await {
            Ok(b) => b,
            Err(_) => return Ok(None),
        };
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|e| Error::Utf8(full, e))
    }

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
        if poll_file(&full, timeout, require_modification)
            .await
            .is_none()
        {
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

    /// Sibling to [`Self::read_json_typed`] but built atop
    /// [`Self::subscribe_json`] — waits for the file to appear (or be
    /// modified, when `require_modification`), then deserializes its
    /// contents into `T`. A timeout becomes
    /// [`Error::LogSubscribeTimedOut`]; a typed-shape deserialize
    /// failure becomes [`Error::TypedDeserialize`] with the file path
    /// attached.
    async fn subscribe_json_typed<T: serde::de::DeserializeOwned>(
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<T, Error> {
        let value = self
            .subscribe_json(dir, stem, timeout, require_modification, None)
            .await?
            .ok_or(Error::LogSubscribeTimedOut)?;
        let full = self.logs_dir().join(dir).join(format!("{stem}.json"));
        serde_json::from_value(value).map_err(|e| Error::TypedDeserialize(full, e))
    }

    /// Polls for a media file (any extension matching `stem.`) and
    /// returns `(mime, base64_payload, filename)` parts. Mirrors
    /// [`Self::read_media_parts_by_stem`] but with create/modify
    /// polling semantics. Returns `None` on deletion or timeout.
    async fn subscribe_media_parts_by_stem(
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Option<(String, String, Option<String>)> {
        use base64::Engine;
        let dir_path = self.logs_dir().join(dir);
        let prefix = format!("{stem}.");

        async fn read_parts(
            path: &std::path::Path,
        ) -> Option<(String, String, Option<String>)> {
            let bytes = tokio::fs::read(path).await.ok()?;
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            let filename =
                path.file_name().and_then(|n| n.to_str()).map(String::from);
            Some((mime, b64, filename))
        }

        let deadline = tokio::time::Instant::now() + timeout;
        let initial_mtime = find_file_mtime_by_prefix(&dir_path, &prefix).await;

        if !require_modification {
            if let Some((path, _)) = &initial_mtime {
                return read_parts(path).await;
            }
        }

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if tokio::time::Instant::now() >= deadline {
                return None;
            }

            let current_mtime =
                find_file_mtime_by_prefix(&dir_path, &prefix).await;
            match (&initial_mtime, &current_mtime) {
                (None, Some((path, _))) => return read_parts(path).await,
                (Some((_, old_t)), Some((path, new_t))) if new_t > old_t => {
                    return read_parts(path).await;
                }
                (Some(_), None) => return None,
                _ => continue,
            }
        }
    }

    async fn subscribe_image_by_stem(
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Option<ImageUrl> {
        let (mime, b64, _) = self
            .subscribe_media_parts_by_stem(
                dir,
                stem,
                timeout,
                require_modification,
            )
            .await?;
        Some(ImageUrl {
            url: format!("data:{mime};base64,{b64}"),
            detail: None,
        })
    }

    async fn subscribe_audio_by_stem(
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Option<InputAudio> {
        let (mime, b64, _) = self
            .subscribe_media_parts_by_stem(
                dir,
                stem,
                timeout,
                require_modification,
            )
            .await?;
        Some(InputAudio {
            data: b64,
            format: mime,
        })
    }

    async fn subscribe_video_by_stem(
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Option<VideoUrl> {
        let (mime, b64, _) = self
            .subscribe_media_parts_by_stem(
                dir,
                stem,
                timeout,
                require_modification,
            )
            .await?;
        Some(VideoUrl {
            url: format!("data:{mime};base64,{b64}"),
        })
    }

    async fn subscribe_file_by_stem(
        &self,
        dir: &str,
        stem: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Option<File> {
        let (_, b64, filename) = self
            .subscribe_media_parts_by_stem(
                dir,
                stem,
                timeout,
                require_modification,
            )
            .await?;
        Some(File {
            file_data: Some(b64),
            filename,
            file_id: None,
            file_url: None,
        })
    }

    pub async fn subscribe_agent_completion(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunkLog,
        Error,
    > {
        self.subscribe_json_typed(
            "agents/completions/response",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_agent_completion_request(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::agent::completions::request::AgentCompletionCreateParamsLog,
        Error,
    > {
        self.subscribe_json_typed(
            "agents/completions/request",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_agent_completion_continuation(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<String, Error> {
        self.subscribe_text(
            "agents/completions/response/continuation",
            id,
            timeout,
            require_modification,
        )
        .await?
        .ok_or(Error::LogSubscribeTimedOut)
    }
    pub async fn subscribe_agent_completion_message_assistant(
        &self,
        id: &str,
        message_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunkLog,
        Error,
    > {
        self.subscribe_json_typed(
            "agents/completions/response/messages/assistant",
            &format!("{id}_{message_index}"),
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_agent_completion_message_tool(
        &self,
        id: &str,
        message_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::agent::completions::response::ToolResponseLog,
        Error,
    > {
        self.subscribe_json_typed(
            "agents/completions/response/messages/tool",
            &format!("{id}_{message_index}"),
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_agent_completion_message_assistant_logprobs(
        &self,
        id: &str,
        message_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<objectiveai_sdk::agent::completions::response::Logprobs, Error> {
        self.subscribe_json_typed(
            "agents/completions/response/messages/assistant/logprobs",
            &format!("{id}_{message_index}"),
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_agent_completion_message_assistant_reasoning(
        &self,
        id: &str,
        message_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<String, Error> {
        // Raw text — `.txt` on disk, not `.json`. See
        // [`Self::read_agent_completion_message_assistant_reasoning`].
        self.subscribe_text(
            "agents/completions/response/messages/assistant/reasoning",
            &format!("{id}_{message_index}"),
            timeout,
            require_modification,
        )
        .await?
        .ok_or(Error::LogSubscribeTimedOut)
    }
    pub async fn subscribe_agent_completion_message_assistant_refusal(
        &self,
        id: &str,
        message_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<String, Error> {
        // Raw text — `.txt` on disk, not `.json`. See
        // [`Self::read_agent_completion_message_assistant_refusal`].
        self.subscribe_text(
            "agents/completions/response/messages/assistant/refusal",
            &format!("{id}_{message_index}"),
            timeout,
            require_modification,
        )
        .await?
        .ok_or(Error::LogSubscribeTimedOut)
    }
    pub async fn subscribe_agent_completion_message_assistant_tool_call(
        &self,
        id: &str,
        message_index: u64,
        tool_call_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<objectiveai_sdk::agent::completions::message::AssistantToolCallDelta, Error> {
        self.subscribe_json_typed(
            "agents/completions/response/messages/assistant/tool_calls",
            &format!("{id}_{message_index}_{tool_call_index}"),
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_agent_completion_message_assistant_image(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<ImageUrl, Error> {
        self.subscribe_image_by_stem(
            "agents/completions/response/messages/assistant/image",
            &format!("{id}_{message_index}_{media_index}"),
            timeout,
            require_modification,
        )
        .await
        .ok_or(Error::LogSubscribeTimedOut)
    }
    pub async fn subscribe_agent_completion_message_assistant_audio(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<InputAudio, Error> {
        self.subscribe_audio_by_stem(
            "agents/completions/response/messages/assistant/audio",
            &format!("{id}_{message_index}_{media_index}"),
            timeout,
            require_modification,
        )
        .await
        .ok_or(Error::LogSubscribeTimedOut)
    }
    pub async fn subscribe_agent_completion_message_assistant_video(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<VideoUrl, Error> {
        self.subscribe_video_by_stem(
            "agents/completions/response/messages/assistant/video",
            &format!("{id}_{message_index}_{media_index}"),
            timeout,
            require_modification,
        )
        .await
        .ok_or(Error::LogSubscribeTimedOut)
    }
    pub async fn subscribe_agent_completion_message_assistant_file(
        &self,
        id: &str,
        message_index: u64,
        media_index: u64,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<File, Error> {
        self.subscribe_file_by_stem(
            "agents/completions/response/messages/assistant/file",
            &format!("{id}_{message_index}_{media_index}"),
            timeout,
            require_modification,
        )
        .await
        .ok_or(Error::LogSubscribeTimedOut)
    }
    pub async fn subscribe_vector_completion(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunkLog,
        Error,
    > {
        self.subscribe_json_typed(
            "vector/completions/response",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_vector_completion_request(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams,
        Error,
    > {
        self.subscribe_json_typed(
            "vector/completions/request",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_function_execution(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunkLog,
        Error,
    > {
        self.subscribe_json_typed(
            "functions/executions/response",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_function_execution_request(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParamsLog,
        Error,
    > {
        self.subscribe_json_typed(
            "functions/executions/request",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_function_execution_retry_token(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<String, Error> {
        self.subscribe_text(
            "functions/executions/response/retry_token",
            id,
            timeout,
            require_modification,
        )
        .await?
        .ok_or(Error::LogSubscribeTimedOut)
    }
    pub async fn subscribe_function_invention(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunkLog,
        Error,
    > {
        self.subscribe_json_typed(
            "functions/inventions/response",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_function_invention_request(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams, Error>
    {
        self.subscribe_json_typed(
            "functions/inventions/request",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_function_invention_recursive(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunkLog,
        Error,
    > {
        self.subscribe_json_typed(
            "functions/inventions/recursive/response",
            id,
            timeout,
            require_modification,
        )
        .await
    }
    pub async fn subscribe_function_invention_recursive_request(
        &self,
        id: &str,
        timeout: std::time::Duration,
        require_modification: bool,
    ) -> Result<
        objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParamsLog,
        Error,
    > {
        self.subscribe_json_typed(
            "functions/inventions/recursive/request",
            id,
            timeout,
            require_modification,
        )
        .await
    }

    // -- Per-agent message queue ---------------------------------------------

    /// Drain every unread row for `spawned_agent_instance_hierarchy` from
    /// `caller_agent_instance_hierarchy`'s perspective and atomically advance the
    /// pair's watermark in `messages_queue`. Each row is hydrated
    /// from its on-disk log file(s) and translated into a typed
    /// [`super::queue::QueueItem`] following the `WORK.md` schema.
    ///
    /// Returns the items in ascending DB-`"index"` order — the same
    /// order they were inserted. First-call semantics inherit from
    /// [`super::super::db::messages::Queue::read_new_messages`]: when
    /// no `messages_queue` row exists yet, the watermark defaults
    /// to 0 and the (typically request-row) index 0 is NOT
    /// returned.
    pub async fn read_new_from_queue(
        &self,
        db: &crate::db::Pool,
        caller_agent_instance_hierarchy: &str,
        spawned_agent_instance_hierarchy: &str,
    ) -> Result<Vec<super::queue::QueueItem>, crate::error::Error> {
        let queue =
            crate::db::messages::Queue::new(db.clone(), self.logs_dir());
        let rows = queue
            .read_new_messages(caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy)
            .await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(self.queue_item_from_row(db, row).await?);
        }
        Ok(items)
    }

    /// Read every queue row for `spawned_agent_instance_hierarchy` (no watermark
    /// filter), advancing `caller_agent_instance_hierarchy`'s watermark to the
    /// returned max. Companion to [`Self::read_new_from_queue`]:
    /// `read_all` returns everything; `read_new` returns only past
    /// the watermark. Both advance the watermark identically — a
    /// subsequent `read_new` after `read_all` always returns empty
    /// until new rows land.
    pub async fn read_all_from_queue(
        &self,
        db: &crate::db::Pool,
        caller_agent_instance_hierarchy: &str,
        spawned_agent_instance_hierarchy: &str,
    ) -> Result<Vec<super::queue::QueueItem>, crate::error::Error> {
        let queue =
            crate::db::messages::Queue::new(db.clone(), self.logs_dir());
        let rows = queue
            .read_all_messages(caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy)
            .await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(self.queue_item_from_row(db, row).await?);
        }
        Ok(items)
    }

    /// Translate one [`crate::db::schema::MessageRow`] into a typed
    /// [`super::queue::QueueItem`] by reading the row's log file(s)
    /// from disk and converting each `LogReference` to a `files`-table
    /// SQL row id (inserted on miss).
    async fn queue_item_from_row(
        &self,
        db: &crate::db::Pool,
        row: crate::db::schema::MessageRow,
    ) -> Result<super::queue::QueueItem, crate::error::Error> {
        use super::queue::QueueItem;
        use objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind;

        let rel_path = crate::db::schema::message_kind_file_path(
            row.kind,
            &row.response_id,
            &row.path,
        );

        match row.kind {
            RequestMessageKind::FunctionExecutionRequest => {
                Ok(QueueItem::FunctionExecutionRequest {
                    id: self.file_id(db, &rel_path).await?,
                })
            }
            RequestMessageKind::FunctionInventionRecursiveRequest => {
                Ok(QueueItem::FunctionInventionRecursiveRequest {
                    id: self.file_id(db, &rel_path).await?,
                })
            }
            RequestMessageKind::AgentCompletionRequest => {
                let envelope: objectiveai_sdk::agent::completions::request::AgentCompletionCreateParamsLog =
                    self.read_log_file(&rel_path).await?;
                let mut messages = Vec::with_capacity(envelope.messages.len());
                for msg_ref in envelope.messages {
                    let msg_log: objectiveai_sdk::agent::completions::message::MessageLog =
                        self.read_log_file(&msg_ref.path).await?;
                    messages.push(
                        self.message_log_to_queue_message(db, msg_log).await?,
                    );
                }
                Ok(QueueItem::AgentCompletionRequest { messages })
            }
            RequestMessageKind::AssistantResponse => {
                let log: objectiveai_sdk::agent::completions::response::streaming::AssistantResponseChunkLog =
                    self.read_log_file(&rel_path).await?;
                Ok(QueueItem::AssistantResponse {
                    reasoning: self.maybe_id(db, log.reasoning).await?,
                    tool_calls: self.maybe_id_list(db, log.tool_calls).await?,
                    content: self.maybe_content(db, log.content).await?,
                    refusal: self.maybe_id(db, log.refusal).await?,
                })
            }
            RequestMessageKind::ToolResponse => {
                let log: objectiveai_sdk::agent::completions::response::ToolResponseLog =
                    self.read_log_file(&rel_path).await?;
                Ok(QueueItem::ToolResponse {
                    tool_call_id: log.tool_call_id,
                    content: self.rich_content_to_content(db, log.content).await?,
                })
            }
            RequestMessageKind::AgentCompletionNotification => {
                let log: objectiveai_sdk::agent::completions::message::RichContentLog =
                    self.read_log_file(&rel_path).await?;
                Ok(QueueItem::Notification {
                    content: self.rich_content_to_content(db, log).await?,
                })
            }
        }
    }

    /// Per-role dispatch from [`objectiveai_sdk::agent::completions::message::MessageLog`]
    /// into [`super::queue::QueueMessage`].
    async fn message_log_to_queue_message(
        &self,
        db: &crate::db::Pool,
        log: objectiveai_sdk::agent::completions::message::MessageLog,
    ) -> Result<super::queue::QueueMessage, crate::error::Error> {
        use super::queue::QueueMessage;
        use objectiveai_sdk::agent::completions::message::MessageLog;

        Ok(match log {
            MessageLog::Developer(m) => QueueMessage::Developer {
                content: self.simple_content_to_content(db, m.content).await?,
                name: m.name,
            },
            MessageLog::System(m) => QueueMessage::System {
                content: self.simple_content_to_content(db, m.content).await?,
                name: m.name,
            },
            MessageLog::User(m) => QueueMessage::User {
                content: self.rich_content_to_content(db, m.content).await?,
                name: m.name,
            },
            MessageLog::Assistant(m) => QueueMessage::Assistant {
                content: self.maybe_content(db, m.content).await?,
                name: m.name,
                reasoning: self.maybe_id(db, m.reasoning).await?,
                tool_calls: self.maybe_id_list(db, m.tool_calls).await?,
                refusal: self.maybe_id(db, m.refusal).await?,
            },
            MessageLog::Tool(m) => QueueMessage::Tool {
                content: self.rich_content_to_content(db, m.content).await?,
                tool_call_id: m.tool_call_id,
            },
        })
    }

    /// Deserialize one log file at `rel_path` (relative to the logs
    /// dir) into the requested type.
    async fn read_log_file<T: serde::de::DeserializeOwned>(
        &self,
        rel_path: &str,
    ) -> Result<T, Error> {
        let full = self.logs_dir().join(rel_path);
        let bytes = tokio::fs::read(&full)
            .await
            .map_err(|e| Error::Read(full.clone(), e))?;
        serde_json::from_slice(&bytes).map_err(|e| Error::Parse(full, e))
    }

    /// Resolve a logs-relative path to its (stable) SQL row id in the
    /// `files` table. Inserts on miss; the `UNIQUE(path)` constraint
    /// keeps one id per path forever.
    async fn file_id(
        &self,
        db: &crate::db::Pool,
        rel_path: &str,
    ) -> Result<i64, crate::error::Error> {
        Ok(crate::db::schema::file_id_for_path(db, rel_path).await?)
    }

    /// Resolve an `Option<LogReference>` to an optional file-id.
    async fn maybe_id(
        &self,
        db: &crate::db::Pool,
        r: Option<objectiveai_sdk::logs::LogReference>,
    ) -> Result<Option<i64>, crate::error::Error> {
        match r {
            Some(r) => Ok(Some(self.file_id(db, &r.path).await?)),
            None => Ok(None),
        }
    }

    /// Resolve a `Vec<LogReference>` to a Vec of file-ids.
    async fn id_list(
        &self,
        db: &crate::db::Pool,
        rs: Vec<objectiveai_sdk::logs::LogReference>,
    ) -> Result<Vec<i64>, crate::error::Error> {
        let mut out = Vec::with_capacity(rs.len());
        for r in rs {
            out.push(self.file_id(db, &r.path).await?);
        }
        Ok(out)
    }

    /// Resolve an `Option<Vec<LogReference>>` to an optional Vec of file-ids.
    async fn maybe_id_list(
        &self,
        db: &crate::db::Pool,
        rs: Option<Vec<objectiveai_sdk::logs::LogReference>>,
    ) -> Result<Option<Vec<i64>>, crate::error::Error> {
        match rs {
            Some(rs) => Ok(Some(self.id_list(db, rs).await?)),
            None => Ok(None),
        }
    }

    /// Translate a [`objectiveai_sdk::agent::completions::message::RichContentLog`]
    /// to [`super::queue::Content`], looking up a file-id for every ref.
    async fn rich_content_to_content(
        &self,
        db: &crate::db::Pool,
        log: objectiveai_sdk::agent::completions::message::RichContentLog,
    ) -> Result<super::queue::Content, crate::error::Error> {
        use super::queue::Content;
        use objectiveai_sdk::agent::completions::message::RichContentLog;
        Ok(match log {
            RichContentLog::Reference(r) => {
                Content::One(self.file_id(db, &r.path).await?)
            }
            RichContentLog::Parts(rs) => Content::Many(self.id_list(db, rs).await?),
        })
    }

    /// Translate a [`objectiveai_sdk::agent::completions::message::SimpleContentLog`]
    /// to [`super::queue::Content`], looking up a file-id for every ref.
    async fn simple_content_to_content(
        &self,
        db: &crate::db::Pool,
        log: objectiveai_sdk::agent::completions::message::SimpleContentLog,
    ) -> Result<super::queue::Content, crate::error::Error> {
        use super::queue::Content;
        use objectiveai_sdk::agent::completions::message::SimpleContentLog;
        Ok(match log {
            SimpleContentLog::Reference(r) => {
                Content::One(self.file_id(db, &r.path).await?)
            }
            SimpleContentLog::Parts(rs) => {
                Content::Many(self.id_list(db, rs).await?)
            }
        })
    }

    /// Resolve `Option<RichContentLog>` to `Option<Content>`.
    async fn maybe_content(
        &self,
        db: &crate::db::Pool,
        log: Option<objectiveai_sdk::agent::completions::message::RichContentLog>,
    ) -> Result<Option<super::queue::Content>, crate::error::Error> {
        match log {
            Some(l) => Ok(Some(self.rich_content_to_content(db, l).await?)),
            None => Ok(None),
        }
    }

    /// Resolve a queue file-id back to its logs-relative path. Returns
    /// `None` if no row matches (e.g. the id was never produced by this
    /// Client, or the `files` table was wiped).
    pub async fn path_for_file_id(
        &self,
        db: &crate::db::Pool,
        id: i64,
    ) -> Result<Option<String>, crate::error::Error> {
        Ok(crate::db::schema::path_for_file_id(db, id).await?)
    }

    /// Resolve a queue file-id to its file content. `.json` files are
    /// parsed into [`LogContent::Json`]; every other extension is
    /// encoded as a `data:` URL via [`LogContent::DataUrl`].
    ///
    /// Errors:
    /// - [`Error::NotFound`] if no `files` row matches `id`.
    /// - [`Error::Read`] if the row exists but the file can't be
    ///   read from disk.
    /// - [`Error::Parse`] if a `.json` file is malformed.
    pub async fn read_file_by_id(
        &self,
        db: &crate::db::Pool,
        id: i64,
    ) -> Result<objectiveai_sdk::cli::command::agents::instances::read::id::Response, crate::error::Error> {
        use objectiveai_sdk::cli::command::agents::instances::read::id::Response as R;
        let rel_path = self
            .path_for_file_id(db, id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("file id {id}")))?;

        // Classify the path via the catalog of writer-side patterns,
        // then dispatch to the matching `read_*` method. The
        // variant-to-method match is the single source of truth for
        // "what `LogContent` shape this kind of file produces" —
        // every existing typed reader already knows its own kind,
        // and `read_file_by_id` re-uses that knowledge instead of
        // re-deriving it from a path-extension sniff.
        let kind =
            super::LogFileKind::from_path(&rel_path).ok_or_else(|| {
                Error::NotFound(format!(
                    "unknown log-file kind for path {rel_path:?}"
                ))
            })?;
        use super::LogFileKind as K;
        // Inner match returns `filesystem::Error`; the trailing
        // `Ok(result?)` lifts it through the `From` impl on
        // `crate::error::Error` once.
        let result: Result<R, Error> = match kind {
            // -- Top-level envelopes (JSON) ---------------------------------
            K::AgentCompletion { id } => self
                .read_agent_completion(&id)
                .await
                .map(R::AgentsCompletionsResponse),
            K::AgentCompletionRequest { id } => self
                .read_agent_completion_request(&id)
                .await
                .map(R::AgentsCompletionsRequest),
            K::AgentCompletionContinuation { id } => self
                .read_agent_completion_continuation(&id)
                .await
                .map(R::Text),
            K::AgentCompletionRequestContinuation { id } => self
                .read_agent_completion_request_continuation(&id)
                .await
                .map(R::Text),
            K::VectorCompletion { id } => self
                .read_vector_completion(&id)
                .await
                .map(R::VectorCompletionsResponse),
            K::VectorCompletionRequest { id } => self
                .read_vector_completion_request(&id)
                .await
                .map(R::VectorCompletionsRequest),
            K::FunctionExecution { id } => self
                .read_function_execution(&id)
                .await
                .map(R::FunctionsExecutionsResponse),
            K::FunctionExecutionRequest { id } => self
                .read_function_execution_request(&id)
                .await
                .map(R::FunctionsExecutionsRequest),
            K::FunctionExecutionRetryToken { id } => self
                .read_function_execution_retry_token(&id)
                .await
                .map(R::Text),
            K::FunctionInvention { id } => self
                .read_function_invention(&id)
                .await
                .map(R::FunctionsInventionsResponse),
            K::FunctionInventionRequest { id } => self
                .read_function_invention_request(&id)
                .await
                .map(R::FunctionsInventionsRequest),
            K::FunctionInventionRecursive { id } => self
                .read_function_invention_recursive(&id)
                .await
                .map(R::FunctionsInventionsRecursiveResponse),
            K::FunctionInventionRecursiveRequest { id } => self
                .read_function_invention_recursive_request(&id)
                .await
                .map(R::FunctionsInventionsRecursiveRequest),

            // -- Per-message metadata (JSON) --------------------------------
            K::AgentCompletionMessageAssistant { id, message_index } => self
                .read_agent_completion_message_assistant(&id, message_index)
                .await
                .map(R::AgentsCompletionsResponseMessagesAssistant),
            K::AgentCompletionMessageTool { id, message_index } => self
                .read_agent_completion_message_tool(&id, message_index)
                .await
                .map(R::AgentsCompletionsResponseMessagesTool),
            K::AgentCompletionMessageAssistantLogprobs { id, message_index } => self
                .read_agent_completion_message_assistant_logprobs(
                    &id,
                    message_index
                )
                .await
                .map(R::AgentsCompletionsResponseMessagesAssistantLogprobs),
            K::AgentCompletionMessageAssistantReasoning { id, message_index } => self
                .read_agent_completion_message_assistant_reasoning(
                    &id,
                    message_index
                )
                .await
                .map(R::Text),
            K::AgentCompletionMessageAssistantRefusal { id, message_index } => self
                .read_agent_completion_message_assistant_refusal(&id, message_index)
                .await
                .map(R::Text),
            K::AgentCompletionMessageAssistantToolCall {
                id,
                message_index,
                tool_call_index,
            } => self
                .read_agent_completion_message_assistant_tool_call(
                    &id,
                    message_index,
                    tool_call_index
                )
                .await
                .map(R::AgentsCompletionsResponseMessagesAssistantToolCalls),

            // -- Assistant content ------------------------------------------
            // Text â†’ `Json(Value::String(text))` so it lands under the
            // same `value.content` shape as every other textual log
            // (reasoning, refusal, etc.). Media â†’ `DataUrl`.
            K::AgentCompletionMessageAssistantText {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_assistant_text(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Text),
            K::AgentCompletionMessageAssistantImage {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_assistant_image(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Image),
            K::AgentCompletionMessageAssistantAudio {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_assistant_audio(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Audio),
            K::AgentCompletionMessageAssistantVideo {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_assistant_video(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Video),
            K::AgentCompletionMessageAssistantFile {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_assistant_file(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::File),

            // -- Tool response content (under .../messages/tool/) -----------
            K::AgentCompletionMessageToolText {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_tool_text(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Text),
            K::AgentCompletionMessageToolImage {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_tool_image(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Image),
            K::AgentCompletionMessageToolAudio {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_tool_audio(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Audio),
            K::AgentCompletionMessageToolVideo {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_tool_video(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Video),
            K::AgentCompletionMessageToolFile {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_message_tool_file(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::File),

            // -- Request-side message content -------------------------------
            K::AgentCompletionRequestMessageText {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_request_message_text(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Text),
            K::AgentCompletionRequestMessageImage {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_request_message_image(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Image),
            K::AgentCompletionRequestMessageAudio {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_request_message_audio(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Audio),
            K::AgentCompletionRequestMessageVideo {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_request_message_video(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::Video),
            K::AgentCompletionRequestMessageFile {
                id,
                message_index,
                media_index,
            } => self
                .read_agent_completion_request_message_file(
                    &id,
                    message_index,
                    media_index,
                )
                .await
                .map(R::File),
            K::AgentCompletionRequestMessage { id, message_index } => self
                .read_agent_completion_request_message(&id, message_index)
                .await
                .map(R::AgentsCompletionsRequestMessages),
            K::AgentCompletionRequestMessageAssistantReasoning {
                id,
                message_index,
            } => self
                .read_agent_completion_request_message_assistant_reasoning(
                    &id,
                    message_index,
                )
                .await
                .map(R::Text),
            K::AgentCompletionRequestMessageAssistantRefusal {
                id,
                message_index,
            } => self
                .read_agent_completion_request_message_assistant_refusal(
                    &id,
                    message_index,
                )
                .await
                .map(R::Text),
            K::AgentCompletionRequestMessageAssistantToolCall {
                id,
                message_index,
                tool_call_index,
            } => self
                .read_agent_completion_request_message_assistant_tool_call(
                    &id,
                    message_index,
                    tool_call_index,
                )
                .await
                .map(R::AgentsCompletionsRequestMessagesAssistantToolCalls),

            // -- Notification content ---------------------------------------
            K::AgentCompletionNotificationText {
                response_id,
                index,
                media_index,
            } => self
                .read_agent_completion_notification_text(
                    &response_id,
                    index,
                    media_index,
                )
                .await
                .map(R::Text),
            K::AgentCompletionNotificationImage {
                response_id,
                index,
                media_index,
            } => self
                .read_agent_completion_notification_image(
                    &response_id,
                    index,
                    media_index,
                )
                .await
                .map(R::Image),
            K::AgentCompletionNotificationAudio {
                response_id,
                index,
                media_index,
            } => self
                .read_agent_completion_notification_audio(
                    &response_id,
                    index,
                    media_index,
                )
                .await
                .map(R::Audio),
            K::AgentCompletionNotificationVideo {
                response_id,
                index,
                media_index,
            } => self
                .read_agent_completion_notification_video(
                    &response_id,
                    index,
                    media_index,
                )
                .await
                .map(R::Video),
            K::AgentCompletionNotificationFile {
                response_id,
                index,
                media_index,
            } => self
                .read_agent_completion_notification_file(
                    &response_id,
                    index,
                    media_index,
                )
                .await
                .map(R::File),

        };
        Ok(result?)
    }

    /// List every direct-child agent of `parent_agent_instance_hierarchy` (one
    /// composite-id segment deeper, no grandchildren) along with
    /// the unix-seconds timestamp of its most recent
    /// `assistant_response` row in the `messages` table.
    /// Newest-first.
    ///
    /// The `agent_instance_hierarchy` in each returned [`ActiveAgent`] is the
    /// sub-portion past the parent — i.e. the trailing
    /// composite-id segment(s) with the `{parent_agent_instance_hierarchy}/`
    /// prefix stripped — so callers can paste it back into
    /// commands that re-prepend the parent (e.g. `agents
    /// read pending`).
    pub async fn list_active(
        &self,
        db: &crate::db::Pool,
        parent_agent_instance_hierarchy: &str,
    ) -> Result<
        Vec<objectiveai_sdk::cli::command::agents::instances::list::ResponseItem>,
        crate::error::Error,
    > {
        use objectiveai_sdk::cli::command::agents::instances::list::ResponseItem;
        let rows = crate::db::schema::list_direct_active_children(
            db,
            parent_agent_instance_hierarchy,
        )
        .await?;
        let prefix = format!("{parent_agent_instance_hierarchy}/");
        Ok(rows
            .into_iter()
            .map(|(full, last_log)| ResponseItem {
                agent_id: full
                    .strip_prefix(&prefix)
                    .unwrap_or(&full)
                    .to_string(),
                last_log,
            })
            .collect())
    }

    /// Whether the cli has ever logged an `agent_completion_request`
    /// row against `agent_instance_hierarchy`. Same predicate that
    /// `read_latest_continuation` uses to distinguish `NoRequests`
    /// from `NoContinuationsFound` / `Found`, but evaluated as a
    /// single `SELECT EXISTS` without walking continuation files.
    pub async fn agent_exists(
        &self,
        db: &crate::db::Pool,
        agent_instance_hierarchy: &str,
    ) -> Result<bool, crate::error::Error> {
        Ok(crate::db::schema::agent_exists(
            db,
            agent_instance_hierarchy,
        )
        .await?)
    }

    /// Stream every queued prompt visible under `parent`. Direct
    /// rows are filtered to direct children of `parent`; tag rows
    /// are always included with the joined tag state from the
    /// `tags` table. Same Vec-of-ResponseItem shape `list_active`
    /// returns.
    pub async fn queue_list(
        &self,
        db: &crate::db::Pool,
        parent: &str,
    ) -> Result<
        Vec<objectiveai_sdk::cli::command::agents::message_queue::read::pending::ResponseItem>,
        crate::error::Error,
    > {
        Ok(crate::db::prompts::list(db, parent).await?)
    }
}

// -- Pure helpers (no &Client) ---------------------------------------------

/// Stem layout for text content (matches `RichContent::extract_media`):
/// `RichContent::Text(_)` produces a single `<id>_<msg>.txt` file (no
/// `media_index`); `RichContent::Parts([... Text { text } ...])` produces
/// one `<id>_<msg>_<part>.txt` file per part.
fn text_stem(id: &str, message_index: u64, media_index: Option<u64>) -> String {
    match media_index {
        None => format!("{id}_{message_index}"),
        Some(mi) => format!("{id}_{message_index}_{mi}"),
    }
}

/// Applies a jq filter to a JSON value, collapsing the multi-result vector
/// the same way the CLI `config get` command does: a single result is
/// unwrapped, an empty result becomes JSON null, and multiple results are
/// wrapped as an array. When `jq` is `None`, the value is returned as-is.
fn apply_jq(
    value: serde_json::Value,
    jq: Option<&str>,
) -> Result<serde_json::Value, Error> {
    let Some(filter) = jq else {
        return Ok(value);
    };
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
                let mtime =
                    tokio::fs::metadata(&path).await.ok()?.modified().ok()?;
                return Some((path, mtime));
            }
        }
    }
    None
}
