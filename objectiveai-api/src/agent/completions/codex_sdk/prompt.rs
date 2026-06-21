//! Translates the upstream's [`Message`] / [`ContinuationItem`] inputs
//! into the JSON shape the Python runner accepts on `--input`, plus
//! materializes any image attachments into the per-request CWD tempdir
//! owned by the client.
//!
//! The runner expects a single user-message JSON object on `--input`:
//!
//! ```text
//! {
//!   "content": "string" | [
//!     {"type": "text",        "text": "..."},
//!     {"type": "local_image", "path": "..."}
//!   ]
//! }
//! ```
//!
//! Codex has no native system role and no system-prompt field. Continuation
//! `UserMessage` items are appended as additional content parts after the
//! original user message.

use std::path::Path;

use serde::{Deserialize, Serialize};

use objectiveai_sdk::agent::completions::message::{
    Message, RichContent, RichContentPart,
};

use super::super::ContinuationItem;

/// The `--input` JSON payload — a single user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunnerUserMessage {
    pub content: Vec<RunnerContentPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// One element of [`RunnerUserMessage::content`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunnerContentPart {
    Text { text: String },
    LocalImage { path: String },
}

/// Output of [`Prompt::new`] — what `super::Client::create` hands to the
/// runner.
#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    /// The `--input` JSON value (a single user message).
    pub input: RunnerUserMessage,
    /// `--resume` value: latest `thread_id` seen in continuation, or
    /// from `request_continuation`. Empty string means "fresh thread".
    pub thread_id: String,
}

fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        "image/svg+xml" => "svg",
        _ => "bin",
    }
}

/// Decode a `data:` URL into raw bytes plus a probable file extension.
/// Only base64-encoded data URLs are supported (the common case for
/// embedded images); raw percent-encoded payloads are rejected.
fn decode_data_url(url: &str) -> Result<(Vec<u8>, &'static str), super::Error> {
    let rest = url.strip_prefix("data:").ok_or_else(|| {
        super::Error::InvalidMessages("data URL must start with `data:`".into())
    })?;
    let (meta, payload) = rest.split_once(',').ok_or_else(|| {
        super::Error::InvalidMessages("data URL is missing `,` separator".into())
    })?;

    let mut mime = "application/octet-stream";
    let mut is_base64 = false;
    for part in meta.split(';') {
        if part == "base64" {
            is_base64 = true;
        } else if part.contains('/') {
            mime = part;
        }
    }

    if !is_base64 {
        return Err(super::Error::InvalidMessages(
            "only base64-encoded data URLs are supported".into(),
        ));
    }

    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .or_else(|_| {
            base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload.trim())
        })
        .or_else(|_| {
            base64::engine::general_purpose::URL_SAFE.decode(payload.trim())
        })
        .or_else(|_| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload.trim())
        })
        .map_err(|e| {
            super::Error::InvalidMessages(format!(
                "data URL base64 decode failed: {e}"
            ))
        })?;

    Ok((bytes, mime_to_ext(mime)))
}

/// Materialize one image (data: or http(s):) into `<cwd>/img-<idx>.<ext>`.
async fn materialize_image(
    cwd: &Path,
    http_client: &reqwest::Client,
    url: &str,
    idx: usize,
) -> Result<String, super::Error> {
    const MAX_BYTES: u64 = 20 * 1024 * 1024; // 20 MiB

    let (bytes, ext) = if url.starts_with("data:") {
        decode_data_url(url)?
    } else if url.starts_with("http://") || url.starts_with("https://") {
        let resp = http_client
            .get(url)
            .send()
            .await
            .map_err(|e| super::Error::ImageFetch(e.to_string()))?
            .error_for_status()
            .map_err(|e| super::Error::ImageFetch(e.to_string()))?;

        if let Some(len) = resp.content_length() {
            if len > MAX_BYTES {
                return Err(super::Error::ImageFetch(format!(
                    "image too large: {len} bytes (max {MAX_BYTES})"
                )));
            }
        }

        let ext = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|ct| mime_to_ext(ct.split(';').next().unwrap_or("").trim()))
            .unwrap_or("bin");

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| super::Error::ImageFetch(e.to_string()))?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(super::Error::ImageFetch(format!(
                "image too large: {} bytes (max {MAX_BYTES})",
                bytes.len()
            )));
        }
        (bytes.to_vec(), ext)
    } else {
        return Err(super::Error::InvalidMessages(format!(
            "unsupported image URL scheme: {url}"
        )));
    };

    let path = cwd.join(format!("img-{idx}.{ext}"));
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| super::Error::Io(e.to_string()))?;
    Ok(path.to_string_lossy().into_owned())
}

async fn push_rich_content(
    cwd: &Path,
    http_client: &reqwest::Client,
    out: &mut Vec<RunnerContentPart>,
    image_idx: &mut usize,
    content: &RichContent,
) -> Result<(), super::Error> {
    match content {
        RichContent::Text(text) => {
            out.push(RunnerContentPart::Text { text: text.clone() });
        }
        RichContent::Parts(parts) => {
            for part in parts {
                match part {
                    RichContentPart::Text { text } => {
                        out.push(RunnerContentPart::Text {
                            text: text.clone(),
                        });
                    }
                    RichContentPart::ImageUrl { image_url } => {
                        let path = materialize_image(
                            cwd,
                            http_client,
                            &image_url.url,
                            *image_idx,
                        )
                        .await?;
                        *image_idx += 1;
                        out.push(RunnerContentPart::LocalImage { path });
                    }
                    RichContentPart::InputAudio { .. } => {
                        return Err(super::Error::InvalidMessages(
                            "audio input is not supported by Codex SDK".into(),
                        ));
                    }
                    RichContentPart::InputVideo { .. }
                    | RichContentPart::VideoUrl { .. } => {
                        return Err(super::Error::InvalidMessages(
                            "video input is not supported by Codex SDK".into(),
                        ));
                    }
                    RichContentPart::File { .. } => {
                        return Err(super::Error::InvalidMessages(
                            "file input is not supported by Codex SDK".into(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

impl Prompt {
    /// Build the runner input from the agent-completions inputs.
    ///
    /// `cwd` is the per-request tempdir owned by the client; image
    /// attachments are written into it and referenced by absolute path.
    /// `http_client` is used to fetch any `http(s):` image URLs.
    pub async fn new(
        cwd: &Path,
        http_client: &reqwest::Client,
        messages: &[Message],
        continuation: Option<&[ContinuationItem<super::State>]>,
        request_continuation: Option<&objectiveai_sdk::agent::codex_sdk::Continuation>,
    ) -> Result<Self, super::Error> {
        let mut user_msg: Option<&objectiveai_sdk::agent::completions::message::UserMessage> =
            None;
        let mut saw_user = false;

        for msg in messages {
            match msg {
                Message::User(u) if !saw_user => {
                    saw_user = true;
                    user_msg = Some(u);
                }
                Message::User(_) => {
                    return Err(super::Error::InvalidMessages(
                        "only one user message is allowed".to_string(),
                    ));
                }
                Message::Assistant(_) => {
                    return Err(super::Error::InvalidMessages(
                        "assistant messages are not allowed".to_string(),
                    ));
                }
                Message::Tool(_) => {
                    return Err(super::Error::InvalidMessages(
                        "tool messages are not allowed".to_string(),
                    ));
                }
            }
        }

        let mut content: Vec<RunnerContentPart> = Vec::new();
        let mut image_idx: usize = 0;

        if let Some(u) = user_msg {
            push_rich_content(
                cwd,
                http_client,
                &mut content,
                &mut image_idx,
                &u.content,
            )
            .await?;
        }

        let session_id = if let Some(items) = continuation {
            let last_state_pos = items
                .iter()
                .rposition(|item| matches!(item, ContinuationItem::State(_)));

            let start = last_state_pos.unwrap_or(0);
            let mut session_id = String::new();

            for (i, item) in items.iter().enumerate() {
                if i < start {
                    continue;
                }
                match item {
                    ContinuationItem::State(state) => {
                        session_id = state.thread_id.clone();
                    }
                    ContinuationItem::ToolMessage(_)
                        if i > start || last_state_pos.is_none() =>
                    {
                        return Err(super::Error::InvalidContinuation(
                            "tool messages must precede a state item".to_string(),
                        ));
                    }
                    ContinuationItem::ToolMessage(_) => {
                        // Tool message at-or-before the most recent state
                        // — handled by an earlier turn.
                    }
                    ContinuationItem::UserMessage(u) => {
                        push_rich_content(
                            cwd,
                            http_client,
                            &mut content,
                            &mut image_idx,
                            &u.content,
                        )
                        .await?;
                    }
                }
            }

            session_id
        } else {
            String::new()
        };

        let thread_id = if session_id.is_empty() {
            request_continuation
                .map(|rc| rc.thread_id.clone())
                .unwrap_or_default()
        } else {
            session_id
        };

        // Empty content is invalid — codex won't accept an empty input.
        if content.is_empty() {
            return Err(super::Error::InvalidMessages(
                "user message has no content".to_string(),
            ));
        }

        Ok(Prompt {
            input: RunnerUserMessage {
                content,
                name: None,
            },
            thread_id,
        })
    }
}
