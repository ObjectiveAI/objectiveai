//! Tool messages.

use super::RichContent;
use serde::{Deserialize, Serialize};

/// Vendor-extension metadata attached to a tool response. The
/// `objectiveai-mcp-proxy` populates known keys (currently
/// `notifications`); the SDK lossy-decodes the MCP `_meta` bag into
/// this typed shape. Unknown keys are dropped.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
)]
pub struct ToolResponseMetadata {
    /// Count of pending notifications the proxy drained and prepended
    /// to the tool response's `content` before returning. Only set
    /// when at least one notification was drained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<u64>,
}

/// A tool message containing the result of a tool call.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct ToolMessage {
    /// The content of the tool response.
    pub content: RichContent,
    /// The ID of the tool call this message responds to.
    pub tool_call_id: String,
    /// Optional vendor-extension metadata, populated by
    /// `objectiveai-mcp-proxy` via MCP's `_meta` extension bag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ToolResponseMetadata>,
}
