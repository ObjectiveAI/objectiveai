//! Codex SDK Agent types and validation logic.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

/// The base configuration for a Codex SDK Agent (without computed ID).
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.codex_sdk.AgentBase")]
pub struct AgentBase {
    /// The upstream provider marker.
    pub upstream: super::Upstream,

    /// The upstream language model identifier (e.g. `gpt-5`).
    pub model: String,

    /// The output mode for vector completions. Ignored for agent completions.
    pub output_mode: super::OutputMode,

    /// Reasoning effort — maps to Codex's `model_reasoning_effort`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub effort: Option<super::Effort>,

    /// Whether this agent may use the codex binary's web-search tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub web_search_enabled: Option<bool>,

    /// Rich content prepended to the user's prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub prefix_content: Option<super::super::completions::message::RichContent>,

    /// Rich content appended after the user's prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub suffix_content: Option<super::super::completions::message::RichContent>,

    /// MCP servers the agent can connect to.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub mcp_servers: Option<super::super::McpServers>,

    /// Laboratories provisioned for the agent — each becomes a
    /// client-side laboratory MCP server whose id DERIVES from the
    /// agent's full id plus the spec (see
    /// [`laboratories::derived_id`](super::super::laboratory::laboratories::derived_id)).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub laboratories: Option<super::super::Laboratories>,



    /// Plugins this agent uses — each IS one MCP server (the
    /// next-iteration plugin shape; see [`super::super::plugin`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub plugins: Vec<super::super::Plugin>,
}

impl AgentBase {
    /// Normalizes the configuration for deterministic ID computation.
    pub fn prepare(&mut self) {
        self.effort = match self.effort.take() {
            Some(effort) => effort.prepare(),
            None => None,
        };
        self.web_search_enabled = match self.web_search_enabled {
            Some(false) => None,
            other => other,
        };
        self.prefix_content = match self.prefix_content.take() {
            Some(prefix_content) if prefix_content.is_empty() => None,
            Some(mut prefix_content) => {
                prefix_content.prepare();
                if prefix_content.is_empty() {
                    None
                } else {
                    Some(prefix_content)
                }
            }
            None => None,
        };
        self.suffix_content = match self.suffix_content.take() {
            Some(suffix_content) if suffix_content.is_empty() => None,
            Some(mut suffix_content) => {
                suffix_content.prepare();
                if suffix_content.is_empty() {
                    None
                } else {
                    Some(suffix_content)
                }
            }
            None => None,
        };
        self.mcp_servers = match self.mcp_servers.take() {
            Some(mcp_servers) => {
                super::super::mcp::mcp_servers::prepare(mcp_servers)
            }
            None => None,
        };
        self.laboratories = match self.laboratories.take() {
            Some(laboratories) => {
                super::super::laboratory::laboratories::prepare(laboratories)
            }
            None => None,
        };
        self.plugins =
            super::super::plugin::prepare(std::mem::take(&mut self.plugins));
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.model.is_empty() {
            return Err("`model` string cannot be empty".to_string());
        }
        if let Some(effort) = &self.effort {
            effort.validate()?;
        }
        if let Some(prefix_content) = &self.prefix_content {
            prefix_content
                .validate_text_or_image_only()
                .map_err(|e| format!("`prefix_content`: {e}"))?;
        }
        if let Some(suffix_content) = &self.suffix_content {
            suffix_content
                .validate_text_or_image_only()
                .map_err(|e| format!("`suffix_content`: {e}"))?;
        }
        if let Some(mcp_servers) = &self.mcp_servers {
            super::super::mcp::mcp_servers::validate(mcp_servers)?;
        }
        if let Some(laboratories) = &self.laboratories {
            super::super::laboratory::laboratories::validate(laboratories)?;
        }
        super::super::plugin::validate(&self.plugins)?;
        Ok(())
    }

    /// Returns prefix content (if set) as a user message, then the provided
    /// messages, then suffix content (if set) as a user message. Codex has
    /// no native system role; system-prompt-style instructions belong on
    /// the user message itself or in the calling layer's input rendering.
    pub fn merged_messages(
        &self,
        messages: Vec<super::super::completions::message::Message>,
    ) -> Vec<super::super::completions::message::Message> {
        use super::super::completions::message::{Message, UserMessage};
        let prefix_len = if self.prefix_content.is_some() { 1 } else { 0 };
        let suffix_len = if self.suffix_content.is_some() { 1 } else { 0 };
        let mut merged =
            Vec::with_capacity(prefix_len + messages.len() + suffix_len);
        if let Some(prefix_content) = &self.prefix_content {
            merged.push(Message::User(UserMessage {
                content: prefix_content.clone(),
            }));
        }
        merged.extend(messages);
        if let Some(suffix_content) = &self.suffix_content {
            merged.push(Message::User(UserMessage {
                content: suffix_content.clone(),
            }));
        }
        merged
    }

    /// Computes the deterministic content-addressed ID.
    pub fn id(&self) -> String {
        let mut hasher = XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(self).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

/// A validated Codex SDK Agent with its computed content-addressed ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.codex_sdk.Agent")]
pub struct Agent {
    /// The deterministic content-addressed ID (22-character base62 string).
    pub id: String,
    /// The normalized configuration.
    #[serde(flatten)]
    pub base: AgentBase,
}

impl TryFrom<AgentBase> for Agent {
    type Error = String;
    fn try_from(mut base: AgentBase) -> Result<Self, Self::Error> {
        base.prepare();
        base.validate()?;
        let id = base.id();
        Ok(Agent { id, base })
    }
}
