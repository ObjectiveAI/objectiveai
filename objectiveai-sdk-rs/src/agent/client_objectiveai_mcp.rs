//! Client-side ObjectiveAI MCP surface declared by an agent.
//!
//! Sits alongside [`super::McpServers`] on every upstream
//! `AgentBase`. Where `mcp_servers` lists full-URL MCP servers the
//! agent will dial out to, this struct declares the
//! ObjectiveAI-managed bits (the built-in `objectiveai-mcp` plus
//! specific plugins / tools by `owner`+`name`+`version`) the agent
//! expects the *calling client* to expose locally back to the API.
//!
//! Content-addressed: the field flows into each upstream's `id()`
//! hash, so swapping in a different plugin reference produces a
//! different agent id.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A single `owner` / `name` / `version` reference identifying one
/// tool inside [`ClientObjectiveaiMcp::tools`]. Plugin references
/// use the larger [`ClientObjectiveaiMcpPluginEntry`] — they carry
/// extra `executable` / `mcp_servers` fields tools don't have.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, JsonSchema, arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.ClientObjectiveaiMcpEntry")]
pub struct ClientObjectiveaiMcpEntry {
    pub owner: String,
    pub name: String,
    pub version: String,
}

impl ClientObjectiveaiMcpEntry {
    /// `owner`, `name`, and `version` must all be non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.owner.is_empty() {
            return Err("`owner` cannot be empty".into());
        }
        if self.name.is_empty() {
            return Err("`name` cannot be empty".into());
        }
        if self.version.is_empty() {
            return Err("`version` cannot be empty".into());
        }
        Ok(())
    }

    /// LLM-visible tool name. See [`materialize_tool_name`].
    pub fn tool_name(&self) -> String {
        materialize_tool_name(&self.owner, &self.name, &self.version)
    }
}

/// Plugin reference inside [`ClientObjectiveaiMcp::plugins`].
///
/// - `owner` / `name` / `version` identify the plugin (same shape as
///   [`ClientObjectiveaiMcpEntry`]).
/// - `executable` controls whether this plugin contributes a tool
///   to the agent's surface (`true`, default) or is loaded purely
///   for its declared MCP servers (`false`). The API's
///   `X-OBJECTIVEAI-TOOLS-ALLOWED` construction honors this: only
///   `executable = true` plugin entries contribute their `name` to
///   the allow-list.
/// - `mcp_servers` selects which of the plugin's manifest-declared
///   `filesystem::plugins::Manifest::mcp_servers` entries should be
///   exposed to the agent, by `name`. `None` ⇒ none of them.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, JsonSchema, arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.ClientObjectiveaiMcpPluginEntry")]
pub struct ClientObjectiveaiMcpPluginEntry {
    pub owner: String,
    pub name: String,
    pub version: String,
    /// `true`: spawn the plugin binary and surface its tools the
    /// usual way. `false`: don't run the plugin; only consume its
    /// declared MCP servers (`mcp_servers` below). Defaults to
    /// `true` so existing declarations keep their current behavior.
    #[serde(default = "default_true")]
    pub executable: bool,
    /// Subset of the plugin's manifest `mcp_servers` to expose, by
    /// `name`. `None` ⇒ none. Names that aren't present in the
    /// plugin's manifest are rejected when the API asks the CLI to
    /// begin them; declarations themselves don't validate the
    /// referent (the plugin may not be installed at declaration
    /// time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_servers: Option<Vec<String>>,
}

fn default_true() -> bool {
    true
}

impl ClientObjectiveaiMcpPluginEntry {
    /// `owner`, `name`, and `version` must all be non-empty; any
    /// `mcp_servers[i]` must also be non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.owner.is_empty() {
            return Err("`owner` cannot be empty".into());
        }
        if self.name.is_empty() {
            return Err("`name` cannot be empty".into());
        }
        if self.version.is_empty() {
            return Err("`version` cannot be empty".into());
        }
        if let Some(names) = self.mcp_servers.as_ref() {
            for n in names {
                if n.is_empty() {
                    return Err("`mcp_servers[i]` cannot be empty".into());
                }
            }
        }
        Ok(())
    }

    /// LLM-visible tool name. See [`materialize_tool_name`].
    pub fn tool_name(&self) -> String {
        materialize_tool_name(&self.owner, &self.name, &self.version)
    }
}

/// Materialize the LLM-visible tool name for an `owner` / `name` /
/// `version` triple: `{owner}-{name}-{version}` with every `.`
/// substituted to `-`. The substitution keeps the result
/// Anthropic-tool-name-regex safe (`^[a-zA-Z0-9_-]{1,128}$`) even
/// when the version field carries semver dots (`1.2.3` -> `1-2-3`).
///
/// Single source of truth shared by [`ClientObjectiveaiMcpEntry::tool_name`],
/// [`crate::filesystem::plugins::Manifest::tool_name`], and
/// [`crate::filesystem::tools::Manifest::tool_name`].
pub fn materialize_tool_name(owner: &str, name: &str, version: &str) -> String {
    format!("{owner}-{name}-{version}").replace('.', "-")
}

/// Client-side MCP surface the agent expects:
///
/// - `objectiveai`: whether the calling client exposes the built-in
///   `objectiveai-mcp`. `None` means unspecified; `Some(true)` /
///   `Some(false)` explicitly opt in / out.
/// - `plugins`: specific plugins (by `owner` / `name` / `version`)
///   plus per-plugin `executable` + `mcp_servers`.
/// - `tools`: specific tools (by `owner` / `name` / `version`).
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, arbitrary::Arbitrary, Default,
)]
#[schemars(rename = "agent.ClientObjectiveaiMcp")]
pub struct ClientObjectiveaiMcp {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub objectiveai: Option<bool>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub plugins: Vec<ClientObjectiveaiMcpPluginEntry>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub tools: Vec<ClientObjectiveaiMcpEntry>,
}

/// Validates the configuration. Each entry's fields must be
/// non-empty, and the `plugins` / `tools` lists each contain no
/// `(owner, name, version)` duplicates. Free-function counterpart to
/// [`super::mcp_servers::validate`].
pub fn validate(this: &ClientObjectiveaiMcp) -> Result<(), String> {
    for entry in &this.plugins {
        entry.validate()?;
    }
    for entry in &this.tools {
        entry.validate()?;
    }
    for (i, a) in this.plugins.iter().enumerate() {
        for b in &this.plugins[i + 1..] {
            if a.owner == b.owner && a.name == b.name && a.version == b.version {
                return Err(format!(
                    "`client_objectiveai_mcp.plugins` contains duplicate entry: \"{}/{}@{}\"",
                    a.owner, a.name, a.version,
                ));
            }
        }
    }
    for (i, a) in this.tools.iter().enumerate() {
        for b in &this.tools[i + 1..] {
            if a == b {
                return Err(format!(
                    "`client_objectiveai_mcp.tools` contains duplicate entry: \"{}/{}@{}\"",
                    a.owner, a.name, a.version,
                ));
            }
        }
    }
    Ok(())
}

/// Sorts plugins + tools for deterministic ordering. Collapses an
/// all-empty struct to `None` so the enclosing `Option` can drop the
/// empty container entirely (same convention as
/// [`super::mcp_servers::prepare`]).
pub fn prepare(mut this: ClientObjectiveaiMcp) -> Option<ClientObjectiveaiMcp> {
    this.plugins.sort();
    this.tools.sort();
    if this.objectiveai.is_none() && this.plugins.is_empty() && this.tools.is_empty() {
        None
    } else {
        Some(this)
    }
}
