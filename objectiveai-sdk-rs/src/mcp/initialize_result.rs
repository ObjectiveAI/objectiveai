//! Types for the MCP `initialize` response.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The server's response to an `initialize` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.InitializeResult")]
pub struct InitializeResult {
    /// The MCP protocol version the server wants to use.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// The server's supported capabilities.
    pub capabilities: ServerCapabilities,
    /// Information about the server implementation.
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,
    /// Optional instructions for LLM integration.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub instructions: Option<String>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}

/// Information about a client or server implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.Implementation")]
pub struct Implementation {
    /// The implementation name.
    pub name: String,
    /// Human-readable title.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub title: Option<String>,
    /// The implementation version.
    pub version: String,
    /// Optional website URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// Optional icons for UI display.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub icons: Option<Vec<super::shared::Icon>>,
}

/// Capabilities that an MCP server may support.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.ServerCapabilities")]
pub struct ServerCapabilities {
    /// Non-standard experimental capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub experimental: Option<IndexMap<String, serde_json::Value>>,
    /// Logging support. Presence indicates the server supports sending log messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub logging: Option<LoggingCapability>,
    /// Completions support. Presence indicates the server supports completions.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub completions: Option<CompletionsCapability>,
    /// Prompt template capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub prompts: Option<PromptsCapability>,
    /// Resource capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub resources: Option<ResourcesCapability>,
    /// Tool capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tools: Option<ToolsCapability>,
    /// Task capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tasks: Option<TasksCapability>,
}

impl ServerCapabilities {
    /// Server declared the objectiveai command-execution extension —
    /// the [`super::OBJECTIVEAI_CAPABILITY`] key in `experimental`.
    /// The key's presence IS the capability; its value is reserved for
    /// future extension settings. When set, the server may push
    /// `notifications/objectiveai/cli_request` frames on the standing
    /// SSE stream, so `Client::connect` opens that stream even when no
    /// list_changed capability asks for it.
    pub fn has_objectiveai(&self) -> bool {
        self.experimental
            .as_ref()
            .is_some_and(|m| m.contains_key(super::OBJECTIVEAI_CAPABILITY))
    }
}

/// Capabilities for prompt templates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.PromptsCapability")]
pub struct PromptsCapability {
    /// Whether the server emits notifications when the prompt list changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Capabilities for resources.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.ResourcesCapability")]
pub struct ResourcesCapability {
    /// Whether the server supports resource subscriptions.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub subscribe: Option<bool>,
    /// Whether the server emits notifications when the resource list changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Capabilities for tools.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.ToolsCapability")]
pub struct ToolsCapability {
    /// Whether the server emits notifications when the tool list changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

/// Marker capability for logging support. Presence indicates the server
/// supports sending log messages to the client.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.LoggingCapability", extend("additionalProperties" = false))]
pub struct LoggingCapability {}

/// Marker capability for completions support. Presence indicates the server
/// supports argument value completions.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.CompletionsCapability", extend("additionalProperties" = false))]
pub struct CompletionsCapability {}

/// Capabilities for task creation and management.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.TasksCapability")]
pub struct TasksCapability {
    /// Present if the server supports listing tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub list: Option<TasksListCapability>,
    /// Present if the server supports cancelling tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub cancel: Option<TasksCancelCapability>,
    /// Task creation capabilities for specific request types.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub requests: Option<TasksRequestsCapability>,
}

/// Marker capability for task listing support.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.TasksListCapability", extend("additionalProperties" = false))]
pub struct TasksListCapability {}

/// Marker capability for task cancellation support.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.TasksCancelCapability", extend("additionalProperties" = false))]
pub struct TasksCancelCapability {}

/// Task creation capabilities scoped to request types.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.TasksRequestsCapability")]
pub struct TasksRequestsCapability {
    /// Task support for tool-related requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tools: Option<TasksToolsCapability>,
}

/// Task capabilities for tool requests.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.TasksToolsCapability")]
pub struct TasksToolsCapability {
    /// Present if tools/call supports task creation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub call: Option<TasksToolsCallCapability>,
}

/// Marker capability for tools/call task creation support.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.TasksToolsCallCapability", extend("additionalProperties" = false))]
pub struct TasksToolsCallCapability {}
