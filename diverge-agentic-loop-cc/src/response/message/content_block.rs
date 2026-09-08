//! What the model produced, one block at a time.

use diverge_provider_sdk::shared::containers::run_loop::response;
use serde::Deserialize;

use super::{
    BashCodeExecutionToolResultContent, Caller,
    CodeExecutionToolResultContent,
    TextEditorCodeExecutionToolResultContent, ToolSearchToolResultContent,
    WebFetchToolResultContent, WebSearchToolResultContent,
};

/// `BetaContentBlock`: one block of an assistant message's content.
///
/// The pinned SDK's four kinds, widened to the current API's fifteen
/// — the server-tool and MCP-connector blocks arrive when those
/// betas are in play, and Claude Code passes them through verbatim.
/// Untagged, the way this crate models unions: each member carries
/// its own `type` literal as a single-variant marker, so the object
/// is self-describing wherever it travels, and only the right
/// variant can accept a given literal. A block newer than the
/// fifteen still fails the parse — deliberately.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ContentBlock {
    /// Text, with whatever citations support it.
    Text {
        /// Always `text`.
        r#type: TextType,
        /// The text itself.
        text: String,
        /// Citations supporting the block; `null` when citations are
        /// not in play at all.
        citations: Option<Vec<TextCitation>>,
    },
    /// The model invoking a tool.
    ToolUse {
        /// Always `tool_use`.
        r#type: ToolUseType,
        /// The call's id, quoted by the matching tool result.
        id: String,
        /// The tool being called.
        name: String,
        /// The arguments, whatever the tool's schema says they are —
        /// `unknown` in the SDK, and kept that way.
        input: serde_json::Value,
        /// Who made the call, when a server-side tool did.
        caller: Option<Caller>,
    },
    /// The model's reasoning, with the signature that lets it be
    /// replayed.
    Thinking {
        /// Always `thinking`.
        r#type: ThinkingType,
        /// The reasoning text.
        thinking: String,
        /// The integrity signature over it.
        signature: String,
    },
    /// Reasoning the API withheld, still replayable.
    RedactedThinking {
        /// Always `redacted_thinking`.
        r#type: RedactedThinkingType,
        /// The opaque encrypted payload.
        data: String,
    },
    /// The model invoking a SERVER-side tool — one Anthropic runs.
    ServerToolUse {
        /// Always `server_tool_use`.
        r#type: ServerToolUseType,
        /// The call's id.
        id: String,
        /// Which server tool — an API-owned vocabulary, open.
        name: String,
        /// The arguments, as the tool's schema says. An object on
        /// every wire the API has shown, kept as a general value the
        /// way every other tool input here is.
        input: serde_json::Value,
        /// Who made the call, when a server-side tool did.
        caller: Option<Caller>,
    },
    /// A web search's answer.
    WebSearchToolResult {
        /// Always `web_search_tool_result`.
        r#type: WebSearchToolResultType,
        /// The results or the error.
        content: WebSearchToolResultContent,
        /// The call being answered.
        tool_use_id: String,
        /// Who made the call, when a server-side tool did.
        caller: Option<Caller>,
    },
    /// A web fetch's answer.
    WebFetchToolResult {
        /// Always `web_fetch_tool_result`.
        r#type: WebFetchToolResultType,
        /// The document or the error.
        content: WebFetchToolResultContent,
        /// The call being answered.
        tool_use_id: String,
        /// Who made the call, when a server-side tool did.
        caller: Option<Caller>,
    },
    /// A code execution's answer.
    CodeExecutionToolResult {
        /// Always `code_execution_tool_result`.
        r#type: CodeExecutionToolResultType,
        /// The result, its encrypted twin, or the error.
        content: CodeExecutionToolResultContent,
        /// The call being answered.
        tool_use_id: String,
    },
    /// A bash execution's answer.
    BashCodeExecutionToolResult {
        /// Always `bash_code_execution_tool_result`.
        r#type: BashCodeExecutionToolResultType,
        /// The result or the error.
        content: BashCodeExecutionToolResultContent,
        /// The call being answered.
        tool_use_id: String,
    },
    /// A text-editor execution's answer.
    TextEditorCodeExecutionToolResult {
        /// Always `text_editor_code_execution_tool_result`.
        r#type: TextEditorCodeExecutionToolResultType,
        /// One of three result shapes, or the error.
        content: TextEditorCodeExecutionToolResultContent,
        /// The call being answered.
        tool_use_id: String,
    },
    /// A tool search's answer.
    ToolSearchToolResult {
        /// Always `tool_search_tool_result`.
        r#type: ToolSearchToolResultType,
        /// The references found, or the error.
        content: ToolSearchToolResultContent,
        /// The call being answered.
        tool_use_id: String,
    },
    /// The model invoking a tool on an MCP CONNECTOR server.
    McpToolUse {
        /// Always `mcp_tool_use`.
        r#type: McpToolUseType,
        /// The call's id.
        id: String,
        /// The arguments, whatever the tool says they are.
        input: serde_json::Value,
        /// The tool being called.
        name: String,
        /// The MCP server it lives on.
        server_name: String,
    },
    /// An MCP connector tool's answer.
    McpToolResult {
        /// Always `mcp_tool_result`.
        r#type: McpToolResultType,
        /// What the tool said: bare text, or blocks — text blocks on
        /// every wire the API has shown, admitted here as any block.
        content: McpToolResultContent,
        /// Whether the tool considers itself to have failed.
        is_error: bool,
        /// The call being answered.
        tool_use_id: String,
    },
    /// A file uploaded to the code-execution container.
    ContainerUpload {
        /// Always `container_upload`.
        r#type: ContainerUploadType,
        /// The file, by id.
        file_id: String,
    },
    /// The API compacted the context here.
    Compaction {
        /// Always `compaction`.
        r#type: CompactionType,
        /// The compaction summary, when carried.
        content: Option<String>,
    },
}

/// An MCP connector tool result's content: a bare string, or blocks.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum McpToolResultContent {
    /// The whole content as one string.
    Text(String),
    /// The content as blocks.
    Blocks(Vec<ContentBlock>),
}

/// The `text` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TextType {
    /// The only value.
    #[default]
    Text,
}

/// The `tool_use` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolUseType {
    /// The only value.
    #[default]
    ToolUse,
}

/// The `thinking` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingType {
    /// The only value.
    #[default]
    Thinking,
}

/// The `redacted_thinking` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RedactedThinkingType {
    /// The only value.
    #[default]
    RedactedThinking,
}

/// The `server_tool_use` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ServerToolUseType {
    /// The only value.
    #[default]
    ServerToolUse,
}

/// The `web_search_tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchToolResultType {
    /// The only value.
    #[default]
    WebSearchToolResult,
}

/// The `web_fetch_tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchToolResultType {
    /// The only value.
    #[default]
    WebFetchToolResult,
}

/// The `code_execution_tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionToolResultType {
    /// The only value.
    #[default]
    CodeExecutionToolResult,
}

/// The `bash_code_execution_tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionToolResultType {
    /// The only value.
    #[default]
    BashCodeExecutionToolResult,
}

/// The `text_editor_code_execution_tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionToolResultType {
    /// The only value.
    #[default]
    TextEditorCodeExecutionToolResult,
}

/// The `tool_search_tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolResultType {
    /// The only value.
    #[default]
    ToolSearchToolResult,
}

/// The `mcp_tool_use` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum McpToolUseType {
    /// The only value.
    #[default]
    McpToolUse,
}

/// The `mcp_tool_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum McpToolResultType {
    /// The only value.
    #[default]
    McpToolResult,
}

/// The `container_upload` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContainerUploadType {
    /// The only value.
    #[default]
    ContainerUpload,
}

/// The `compaction` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CompactionType {
    /// The only value.
    #[default]
    Compaction,
}

/// `BetaTextCitation`: where a piece of text came from.
///
/// Five location vocabularies for five source shapes: character
/// offsets into plain text, pages of a PDF, block indices of custom
/// content, web search hits, and search-result blocks. The `Param`
/// unions on the request side carry the same fields (minus
/// `file_id`), so these serve both directions. Untagged with literal
/// markers, like every union here.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum TextCitation {
    /// A span of characters in a plain-text document.
    CharLocation {
        /// Always `char_location`.
        r#type: CharLocationType,
        /// The text being cited.
        cited_text: String,
        /// Which document, by request order.
        document_index: u64,
        /// The document's title, if it had one.
        document_title: Option<String>,
        /// The cited file's id, when citations point into uploaded
        /// files. Response-side only; the request-side params never
        /// carry it, which is why it skips when absent.
        file_id: Option<String>,
        /// First cited character.
        start_char_index: u64,
        /// One past the last cited character.
        end_char_index: u64,
    },
    /// A span of pages in a PDF.
    PageLocation {
        /// Always `page_location`.
        r#type: PageLocationType,
        /// The text being cited.
        cited_text: String,
        /// Which document, by request order.
        document_index: u64,
        /// The document's title, if it had one.
        document_title: Option<String>,
        /// The cited file's id, when citations point into uploaded
        /// files. Response-side only; the request-side params never
        /// carry it, which is why it skips when absent.
        file_id: Option<String>,
        /// First cited page.
        start_page_number: u64,
        /// One past the last cited page.
        end_page_number: u64,
    },
    /// A span of blocks in custom content.
    ContentBlockLocation {
        /// Always `content_block_location`.
        r#type: ContentBlockLocationType,
        /// The text being cited.
        cited_text: String,
        /// Which document, by request order.
        document_index: u64,
        /// The document's title, if it had one.
        document_title: Option<String>,
        /// The cited file's id, when citations point into uploaded
        /// files. Response-side only; the request-side params never
        /// carry it, which is why it skips when absent.
        file_id: Option<String>,
        /// First cited block.
        start_block_index: u64,
        /// One past the last cited block.
        end_block_index: u64,
    },
    /// A web search hit, cited by encrypted index.
    WebSearchResultLocation {
        /// Always `web_search_result_location`.
        r#type: WebSearchResultLocationType,
        /// The text being cited.
        cited_text: String,
        /// The hit, by encrypted index.
        encrypted_index: String,
        /// The hit's title, if it had one.
        title: Option<String>,
        /// The hit's URL.
        url: String,
    },
    /// A search-result block, cited by position.
    SearchResultLocation {
        /// Always `search_result_location`.
        r#type: SearchResultLocationType,
        /// The text being cited.
        cited_text: String,
        /// One past the last cited block.
        end_block_index: u64,
        /// Which search result, by request order.
        search_result_index: u64,
        /// The result's source.
        source: String,
        /// First cited block.
        start_block_index: u64,
        /// The result's title, if it had one.
        title: Option<String>,
    },
}

/// The `char_location` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CharLocationType {
    /// The only value.
    #[default]
    CharLocation,
}

/// The `page_location` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PageLocationType {
    /// The only value.
    #[default]
    PageLocation,
}

/// The `content_block_location` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockLocationType {
    /// The only value.
    #[default]
    ContentBlockLocation,
}

/// The `web_search_result_location` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchResultLocationType {
    /// The only value.
    #[default]
    WebSearchResultLocation,
}

/// The `search_result_location` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultLocationType {
    /// The only value.
    #[default]
    SearchResultLocation,
}

impl ContentBlock {
    /// This block, as the chunk it is — the assistant-side leaf.
    ///
    /// What speaks, and what stays silent, deliberately:
    ///
    /// - text becomes the text chunk, its citations dropped — the
    ///   chunk vocabulary has no citation home;
    /// - thinking becomes the reasoning chunk, its signature
    ///   dropped — same reason, and the api crate's choice too;
    /// - the three tool-use kinds — client, server, MCP connector —
    ///   become tool-call chunks, the arguments as the JSON they
    ///   already are (the api crate emits all three as calls);
    /// - redacted thinking is opaque by construction; the
    ///   server-tool RESULT kinds, container uploads and compaction
    ///   markers are the API's own loop narrating itself — not the
    ///   container's tool story — and the api crate drops every one
    ///   of them.
    pub fn into_chunks(
        self,
        chunks: &mut Vec<response::AgenticLoopChunk>,
        parent_tool_call_id: Option<&str>,
    ) {
        let parent_tool_call_id = parent_tool_call_id.map(str::to_string);
        match self {
            ContentBlock::Text { text, .. } => {
                chunks.push(
                    response::AgenticLoopChunk::AssistantTextContent(
                        response::AssistantTextContentChunk {
                            r#type: Default::default(),
                            parent_tool_call_id,
                            logprobs: None,
                            inner: rmcp::model::TextContent::new(text),
                        },
                    ),
                );
            }
            ContentBlock::Thinking { thinking, .. } => {
                chunks.push(
                    response::AgenticLoopChunk::AssistantReasoning(
                        response::AssistantReasoningChunk {
                            r#type: Default::default(),
                            parent_tool_call_id,
                            logprobs: None,
                            inner: rmcp::model::TextContent::new(thinking),
                        },
                    ),
                );
            }
            ContentBlock::ToolUse {
                id, name, input, ..
            }
            | ContentBlock::ServerToolUse {
                id, name, input, ..
            }
            | ContentBlock::McpToolUse {
                id, name, input, ..
            } => {
                chunks.push(
                    response::AgenticLoopChunk::AssistantToolCall(
                        response::AssistantToolCallChunk {
                            r#type: Default::default(),
                            parent_tool_call_id,
                            id,
                            meta: None,
                            name,
                            arguments: Some(input.to_string()),
                        },
                    ),
                );
            }
            ContentBlock::RedactedThinking { .. }
            | ContentBlock::WebSearchToolResult { .. }
            | ContentBlock::WebFetchToolResult { .. }
            | ContentBlock::CodeExecutionToolResult { .. }
            | ContentBlock::BashCodeExecutionToolResult { .. }
            | ContentBlock::TextEditorCodeExecutionToolResult { .. }
            | ContentBlock::ToolSearchToolResult { .. }
            | ContentBlock::McpToolResult { .. }
            | ContentBlock::ContainerUpload { .. }
            | ContentBlock::Compaction { .. } => {}
        }
    }
}
