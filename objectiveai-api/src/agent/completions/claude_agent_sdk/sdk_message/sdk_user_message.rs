use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SDKUserMessageType {
    User,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageParamRole {
    User,
    Assistant,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageParamContent {
    String(String),
    Blocks(Vec<super::super::content_block_param::ContentBlockParam>),
}

impl MessageParamContent {
    pub fn push(&mut self, block: super::super::content_block_param::ContentBlockParam) {
        match self {
            MessageParamContent::String(s) => {
                let mut blocks = if s.is_empty() {
                    vec![]
                } else {
                    vec![super::super::content_block_param::ContentBlockParam::Text(
                        super::super::content_block_param::TextBlockParam {
                            text: std::mem::take(s),
                            r#type: super::super::content_block_param::TextBlockParamType::Text,
                            cache_control: None,
                            citations: None,
                        },
                    )]
                };
                blocks.push(block);
                *self = MessageParamContent::Blocks(blocks);
            }
            MessageParamContent::Blocks(blocks) => {
                blocks.push(block);
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MessageParam {
    pub content: MessageParamContent,
    pub role: MessageParamRole,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SDKUserMessage {
    pub r#type: SDKUserMessageType,
    pub message: MessageParam,
    pub parent_tool_use_id: Option<String>,
    #[serde(rename = "isSynthetic", skip_serializing_if = "Option::is_none")]
    pub is_synthetic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    // The SDK's `UserMessage` dataclass carries no `session_id`, so the Python
    // runner never emits one for tool-result user messages. Optional, or those
    // lines fail to deserialize and are silently dropped before logging.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl SDKUserMessage {
    /// Transforms this upstream user message into a downstream
    /// [`AgentCompletionChunk`] carrying one tool response per `tool_result`,
    /// or `None` when the message is not a tool result.
    ///
    /// The tool-call id and content come from the `tool_result` block(s) in
    /// `message.content` — the shape the runner actually emits for a direct
    /// tool call (which carries NO `parent_tool_use_id`). As a fallback, a
    /// structured top-level `tool_use_result` keyed by `parent_tool_use_id`
    /// (sub-agent / Task results that arrive without an inline block) is still
    /// honored.
    #[allow(clippy::too_many_arguments)]
    pub fn into_downstream(
        self,
        id: String,
        created: u64,
        message_index: u64,
        upstream: objectiveai_sdk::agent::Upstream,
        agent_instance_hierarchy: String,
        agent_id: String,
        agent_full_id: String,
        agent_remote: Option<objectiveai_sdk::RemotePath>,
    ) -> Option<objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk> {
        let mut messages = Vec::new();

        // Primary: inline `tool_result` blocks (direct tool calls). The
        // tool-call id is the block's own `tool_use_id`.
        if let MessageParamContent::Blocks(blocks) = &self.message.content {
            for block in blocks {
                if let super::super::content_block_param::ContentBlockParam::ToolResult(tr) =
                    block
                {
                    messages.push(tool_response_chunk(
                        message_index,
                        tr.tool_use_id.clone(),
                        render_tool_result_content(tr.content.as_ref()),
                    ));
                }
            }
        }

        // Fallback: a structured `tool_use_result` keyed by
        // `parent_tool_use_id` (sub-agent / Task results with no inline
        // `tool_result` block).
        if messages.is_empty() {
            if let (Some(tool_use_result), Some(tool_call_id)) =
                (&self.tool_use_result, &self.parent_tool_use_id)
            {
                messages.push(tool_response_chunk(
                    message_index,
                    tool_call_id.clone(),
                    objectiveai_sdk::agent::completions::message::RichContent::Text(
                        serde_json::to_string(tool_use_result).unwrap_or_default(),
                    ),
                ));
            }
        }

        if messages.is_empty() {
            return None;
        }

        Some(
            objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk {
                id,
                agent_instance_hierarchy,
                agent_id,
                agent_full_id,
                agent_remote,
                created,
                messages,
                object: Default::default(),
                usage: None,
                upstream,
                error: None,
                continuation: None,
                messages_queued: None,
            },
        )
    }
}

/// Builds one tool-response message chunk.
fn tool_response_chunk(
    index: u64,
    tool_call_id: String,
    content: objectiveai_sdk::agent::completions::message::RichContent,
) -> objectiveai_sdk::agent::completions::response::streaming::MessageChunk {
    objectiveai_sdk::agent::completions::response::streaming::MessageChunk::Tool(
        objectiveai_sdk::agent::completions::response::ToolResponse {
            role: Default::default(),
            index,
            inner: objectiveai_sdk::agent::completions::message::ToolMessage {
                content,
                tool_call_id,
                metadata: None,
            },
            request_message_ids: None,
        },
    )
}

/// Renders a `tool_result` block's content to [`RichContent`],
/// PRESERVING every block kind the Claude Agent SDK can deliver —
/// text, images, documents, search results, and tool references —
/// rather than the old text-only collapse that silently dropped an
/// agent's tool-returned media (images especially) before it reached
/// the log writer. Emits `Text` for a bare string or an all-text
/// block list; otherwise `Parts`. `None` → empty text.
fn render_tool_result_content(
    content: Option<&super::super::content_block_param::ToolResultBlockParamContent>,
) -> objectiveai_sdk::agent::completions::message::RichContent {
    use super::super::content_block_param::ToolResultBlockParamContent;
    use objectiveai_sdk::agent::completions::message::RichContent;
    match content {
        None => RichContent::Text(String::new()),
        Some(ToolResultBlockParamContent::String(s)) => {
            RichContent::Text(s.clone())
        }
        Some(ToolResultBlockParamContent::Blocks(blocks)) => {
            let parts: Vec<_> = blocks
                .iter()
                .filter_map(tool_result_block_to_part)
                .collect();
            // Collapse an all-text (or empty) result back to a plain
            // Text carrier; keep Parts when any media rode along.
            if parts.iter().all(|p| {
                matches!(
                    p,
                    objectiveai_sdk::agent::completions::message::RichContentPart::Text { .. }
                )
            }) {
                let text = parts
                    .iter()
                    .filter_map(|p| match p {
                        objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                            text,
                        } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>();
                RichContent::Text(text)
            } else {
                RichContent::Parts(parts)
            }
        }
    }
}

/// Map one Claude Agent SDK tool-result content block to a
/// [`RichContentPart`]. Every variant is preserved: images become
/// `ImageUrl` (base64 → data URL, or the remote url); documents
/// become `File` (URL/base64 PDF) or `Text` (plain-text / inlined
/// content-block docs); search results flatten to `Text` (title +
/// body); tool references render as a `[tool: name]` marker.
fn tool_result_block_to_part(
    block: &super::super::content_block_param::ToolResultContentBlockParam,
) -> Option<objectiveai_sdk::agent::completions::message::RichContentPart> {
    use super::super::content_block_param::{
        DocumentSource, ImageSource, ToolResultContentBlockParam,
    };
    use objectiveai_sdk::agent::completions::message::{
        File, ImageUrl, RichContentPart,
    };
    let text_part = |text: String| RichContentPart::Text { text };
    match block {
        ToolResultContentBlockParam::Text(t) => {
            Some(text_part(t.text.clone()))
        }
        ToolResultContentBlockParam::Image(img) => {
            let url = match &img.source {
                ImageSource::Base64(b64) => {
                    format!(
                        "data:{};base64,{}",
                        image_media_type(&b64.media_type),
                        b64.data,
                    )
                }
                ImageSource::URL(u) => u.url.clone(),
            };
            Some(RichContentPart::ImageUrl {
                image_url: ImageUrl { url, detail: None },
            })
        }
        ToolResultContentBlockParam::Document(doc) => match &doc.source {
            DocumentSource::URLPDF(u) => Some(RichContentPart::File {
                file: File {
                    file_data: None,
                    file_id: None,
                    file_url: Some(u.url.clone()),
                    filename: doc.title.clone(),
                },
            }),
            DocumentSource::Base64PDF(b64) => Some(RichContentPart::File {
                file: File {
                    file_data: Some(b64.data.clone()),
                    file_id: None,
                    file_url: None,
                    filename: doc.title.clone(),
                },
            }),
            DocumentSource::PlainText(pt) => Some(text_part(pt.data.clone())),
            DocumentSource::ContentBlock(cb) => {
                Some(text_part(content_block_source_text(&cb.content)))
            }
        },
        ToolResultContentBlockParam::SearchResult(sr) => {
            // A genuine representation of how the model receives a
            // search result: the `<search_result>` element carrying
            // its source and title, wrapping the body.
            let body = sr
                .content
                .iter()
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            Some(text_part(format!(
                "<search_result source=\"{}\" title=\"{}\">\n{}\n</search_result>",
                sr.source, sr.title, body,
            )))
        }
        ToolResultContentBlockParam::ToolReference(tr) => {
            // As the model receives a referenced tool: a self-closing
            // `<tool_reference>` naming it.
            Some(text_part(format!(
                "<tool_reference name=\"{}\" />",
                tr.tool_name,
            )))
        }
    }
}

/// The `image/<subtype>` MIME string for a base64 image source's
/// media type (for the data URL).
fn image_media_type(
    media_type: &super::super::content_block_param::Base64ImageSourceMediaType,
) -> &'static str {
    use super::super::content_block_param::Base64ImageSourceMediaType as MT;
    match media_type {
        MT::ImageJpeg => "image/jpeg",
        MT::ImagePng => "image/png",
        MT::ImageGif => "image/gif",
        MT::ImageWebp => "image/webp",
    }
}

/// Flatten a document `ContentBlock` source's data to text (its text
/// blocks joined; inlined images are noted as markers).
fn content_block_source_text(
    data: &super::super::content_block_param::ContentBlockSourceData,
) -> String {
    use super::super::content_block_param::{
        ContentBlockSourceContent, ContentBlockSourceData,
    };
    match data {
        ContentBlockSourceData::String(s) => s.clone(),
        ContentBlockSourceData::Blocks(blocks) => blocks
            .iter()
            .map(|b| match b {
                ContentBlockSourceContent::Text(t) => t.text.clone(),
                ContentBlockSourceContent::Image(_) => "[image]".to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}
