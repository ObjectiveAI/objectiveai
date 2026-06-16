//! Translates the canonical agent-completions [`Message`] /
//! [`ContinuationItem`] inputs into the runner's `messages` list (the
//! full conversation in the runner's own wire shape), plus materializes
//! any image attachments as data URLs the runner can consume.
//!
//! The gemini runner is STATELESS: it receives the FULL conversation on
//! every `run` request and holds no session. The public continuation
//! persists the conversation as canonical
//! [`completions::message::Message`]s; this builder replays that prior
//! history (`request_continuation.messages`) followed by this turn's
//! messages (the merged `messages` argument + continuation `UserMessage`
//! / `ToolMessage` items), translating each canonical message into the
//! runner's wire [`super::Message`] shape.
//!
//! [`Prompt::new`] returns both:
//! - `messages`: the runner-wire conversation sent on the request, and
//! - `canonical_history`: the same conversation in the canonical shape
//!   (prior history + this turn's inputs), which the client extends with
//!   the model turn it produces and stores as the new continuation
//!   history.
//!
//! Mapping of canonical message roles → runner message roles:
//! - `System` / `Developer` → folded into a single leading
//!   `system_prompt` string (the runner has a dedicated
//!   `system_instruction`); not emitted as `messages` items.
//! - `User` → runner `user` message with text / image parts.
//! - `Assistant` → runner `model` message (text parts + tool_calls).
//! - `Tool` → runner `tool` message.
//!
//! [`completions::message::Message`]:
//!     objectiveai_sdk::agent::completions::message::Message

use objectiveai_sdk::agent::completions::message::{
    AssistantToolCall, Message, RichContent, RichContentPart, SimpleContent,
    SimpleContentPart,
};

use super::super::ContinuationItem;
use super::{ContentPart, ToolCall};

/// Output of [`Prompt::new`] — what `super::Client::create` hands to the
/// runner.
#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    /// The full conversation in runner-message shape (prior history +
    /// this turn). Sent as `params.messages`.
    pub messages: Vec<super::Message>,
    /// Folded system / developer text. Empty string means none.
    pub system_prompt: String,
    /// The same conversation in the canonical message shape (prior
    /// history + this turn's inputs). The client extends this with the
    /// model turn the runner produced and stores it as the new
    /// continuation history.
    pub canonical_history: Vec<Message>,
}

fn simple_content_to_text(content: &SimpleContent) -> String {
    match content {
        SimpleContent::Text(s) => s.clone(),
        SimpleContent::Parts(parts) => parts
            .iter()
            .map(|p| match p {
                SimpleContentPart::Text { text } => text.as_str(),
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}

/// Convert one [`RichContent`] into runner [`ContentPart`]s, resolving
/// `http(s):` image URLs to data URLs via `http_client`. Text parts are
/// passed through; data-URL image parts are passed through verbatim.
async fn rich_content_to_parts(
    http_client: &reqwest::Client,
    content: &RichContent,
) -> Result<Vec<ContentPart>, super::Error> {
    let mut out = Vec::new();
    match content {
        RichContent::Text(text) => {
            out.push(ContentPart::Text { text: text.clone() });
        }
        RichContent::Parts(parts) => {
            for part in parts {
                match part {
                    RichContentPart::Text { text } => {
                        out.push(ContentPart::Text { text: text.clone() });
                    }
                    RichContentPart::ImageUrl { image_url } => {
                        let url = resolve_image_url(
                            http_client,
                            &image_url.url,
                        )
                        .await?;
                        out.push(ContentPart::Image {
                            url,
                            mime_type: None,
                        });
                    }
                    RichContentPart::InputAudio { .. } => {
                        return Err(super::Error::InvalidMessages(
                            "audio input is not supported by Gemini".into(),
                        ));
                    }
                    RichContentPart::InputVideo { .. }
                    | RichContentPart::VideoUrl { .. } => {
                        return Err(super::Error::InvalidMessages(
                            "video input is not supported by Gemini".into(),
                        ));
                    }
                    RichContentPart::File { .. } => {
                        return Err(super::Error::InvalidMessages(
                            "file input is not supported by Gemini".into(),
                        ));
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Resolve an image URL into a form the runner accepts. `data:` URLs are
/// passed through verbatim; `http(s):` URLs are fetched and re-encoded as
/// a base64 `data:` URL so the stateless runner doesn't need outbound
/// network access of its own.
async fn resolve_image_url(
    http_client: &reqwest::Client,
    url: &str,
) -> Result<String, super::Error> {
    const MAX_BYTES: u64 = 20 * 1024 * 1024; // 20 MiB

    if url.starts_with("data:") {
        return Ok(url.to_string());
    }
    if url.starts_with("http://") || url.starts_with("https://") {
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

        let mime = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.split(';').next().unwrap_or("").trim().to_string())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "application/octet-stream".to_string());

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
        use base64::Engine as _;
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(&bytes);
        return Ok(format!("data:{mime};base64,{encoded}"));
    }
    Err(super::Error::InvalidMessages(format!(
        "unsupported image URL scheme: {url}"
    )))
}

/// Convert SDK [`AssistantToolCall`]s into runner [`ToolCall`]s. The
/// function `arguments` JSON string is parsed into a `Value`; on parse
/// failure it's left as `Null` so the call still round-trips.
fn assistant_tool_calls(
    tool_calls: &Option<Vec<AssistantToolCall>>,
) -> Vec<ToolCall> {
    let mut out = Vec::new();
    if let Some(calls) = tool_calls {
        for call in calls {
            let AssistantToolCall::Function { id, function } = call;
            let args: serde_json::Value =
                serde_json::from_str(&function.arguments)
                    .unwrap_or(serde_json::Value::Null);
            out.push(ToolCall {
                id: id.clone(),
                name: function.name.clone(),
                args,
            });
        }
    }
    out
}

/// Flatten a [`RichContent`] to a plain string for tool-result content
/// (the runner accepts a string body for `tool` messages).
fn rich_content_to_text(content: &RichContent) -> String {
    match content {
        RichContent::Text(s) => s.clone(),
        RichContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                RichContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}

impl Prompt {
    /// Build the full runner conversation from the agent-completions
    /// inputs.
    ///
    /// - `messages`: this turn's merged messages (system / user /
    ///   assistant / tool). On resumption this is typically empty (the
    ///   orchestrator passes continuation items instead).
    /// - `continuation`: the in-process continuation items
    ///   (State / UserMessage / ToolMessage) appended this round.
    /// - `request_continuation`: the wire continuation, whose
    ///   `messages` field carries the FULL prior conversation history in
    ///   the canonical shape.
    ///
    /// `http_client` resolves any `http(s):` image URLs into data URLs.
    pub async fn new(
        http_client: &reqwest::Client,
        messages: &[Message],
        continuation: Option<&[ContinuationItem<super::State>]>,
        request_continuation: Option<
            &objectiveai_sdk::agent::gemini::Continuation,
        >,
    ) -> Result<Self, super::Error> {
        // The canonical conversation we replay: prior history (verbatim
        // from the wire continuation) followed by this turn's inputs.
        let mut canonical_history: Vec<Message> = request_continuation
            .map(|rc| rc.messages.clone())
            .unwrap_or_default();

        // This turn's merged messages go onto the canonical history.
        canonical_history.extend(messages.iter().cloned());

        // Continuation items appended this round.
        //
        // Items at or before the most recent State were already folded
        // into the prior turn's history (and replayed via
        // `request_continuation.messages`); only items AFTER the last
        // State belong to this turn. On a fresh in-process resume with
        // no State yet, every item is new. Mirror codex's continuation
        // ordering validation (tool messages must precede a state item).
        if let Some(items) = continuation {
            let last_state_pos = items
                .iter()
                .rposition(|item| matches!(item, ContinuationItem::State(_)));
            let start = match last_state_pos {
                Some(pos) => pos + 1,
                None => 0,
            };
            for (i, item) in items.iter().enumerate() {
                match item {
                    ContinuationItem::State(_) => {}
                    ContinuationItem::ToolMessage(t) => {
                        if i >= start {
                            canonical_history.push(Message::Tool(t.clone()));
                        } else if last_state_pos.is_none() {
                            return Err(super::Error::InvalidContinuation(
                                "tool messages must precede a state item"
                                    .to_string(),
                            ));
                        }
                        // Tool message at-or-before the most recent state
                        // — already folded into the replayed history.
                    }
                    ContinuationItem::UserMessage(u) => {
                        if i >= start {
                            canonical_history
                                .push(Message::User(u.clone()));
                        }
                    }
                }
            }
        }

        // Translate the canonical conversation into the runner's wire
        // shape, folding every system / developer message into a single
        // leading `system_prompt` string.
        let mut out: Vec<super::Message> = Vec::new();
        let mut system_parts: Vec<String> = Vec::new();

        for msg in &canonical_history {
            match msg {
                Message::System(sys) => {
                    let text = simple_content_to_text(&sys.content);
                    if !text.is_empty() {
                        system_parts.push(text);
                    }
                }
                Message::Developer(dev) => {
                    let text = simple_content_to_text(&dev.content);
                    if !text.is_empty() {
                        system_parts.push(text);
                    }
                }
                Message::User(u) => {
                    let content =
                        rich_content_to_parts(http_client, &u.content).await?;
                    if !content.is_empty() {
                        out.push(super::Message::User { content });
                    }
                }
                Message::Assistant(a) => {
                    let content = match &a.content {
                        Some(c) => {
                            rich_content_to_parts(http_client, c).await?
                        }
                        None => Vec::new(),
                    };
                    let tool_calls = assistant_tool_calls(&a.tool_calls);
                    out.push(super::Message::Model {
                        content,
                        tool_calls,
                    });
                }
                Message::Tool(t) => {
                    out.push(super::Message::Tool {
                        tool_call_id: t.tool_call_id.clone(),
                        name: String::new(),
                        content: rich_content_to_text(&t.content),
                        is_error: false,
                    });
                }
            }
        }

        // The runner rejects an empty conversation.
        if out.is_empty() {
            return Err(super::Error::InvalidMessages(
                "conversation has no messages".to_string(),
            ));
        }

        Ok(Prompt {
            messages: out,
            system_prompt: system_parts.join("\n\n"),
            canonical_history,
        })
    }
}
