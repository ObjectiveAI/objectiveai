//! Gemini Agent types and validation logic.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

/// The base configuration for a Gemini Agent (without computed ID).
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
#[schemars(rename = "agent.gemini.AgentBase")]
pub struct AgentBase {
    /// The upstream provider marker.
    pub upstream: super::Upstream,

    /// The upstream language model identifier.
    pub model: String,

    /// The output mode for vector completions. Ignored for agent completions.
    pub output_mode: super::OutputMode,

    /// Whether thinking/extended thinking is enabled.
    ///
    /// Defaults to `true`. Set to `false` to disable.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub thinking: Option<bool>,

    /// The effort level for model output.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub effort: Option<super::Effort>,

    /// Whether this agent may use the web-search tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub web_search_enabled: Option<bool>,

    /// System prompt for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub system_prompt: Option<String>,

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

    /// Client-side ObjectiveAI MCP surface the calling client is
    /// expected to expose locally back to the API (objectiveai
    /// built-in, plus specific plugins / tools by owner+name+version).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub client_objectiveai_mcp: Option<super::super::ClientObjectiveaiMcp>,
}

impl AgentBase {
    /// Normalizes the configuration for deterministic ID computation.
    pub fn prepare(&mut self) {
        self.thinking = match self.thinking {
            Some(true) => None,
            other => other,
        };
        self.effort = match self.effort.take() {
            Some(effort) => effort.prepare(),
            None => None,
        };
        self.web_search_enabled = match self.web_search_enabled {
            Some(false) => None,
            other => other,
        };
        self.system_prompt = match self.system_prompt.take() {
            Some(system_prompt) if system_prompt.is_empty() => None,
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
        self.client_objectiveai_mcp = match self.client_objectiveai_mcp.take() {
            Some(cm) => super::super::client_objectiveai_mcp::prepare(cm),
            None => None,
        };
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
        if let Some(cm) = &self.client_objectiveai_mcp {
            super::super::client_objectiveai_mcp::validate(cm)?;
        }
        Ok(())
    }

    /// Returns system prompt (if set) as a system message, then prefix content
    /// (if set) as a user message, then the provided messages, then suffix
    /// content (if set) as a user message.
    pub fn merged_messages(
        &self,
        messages: Vec<super::super::completions::message::Message>,
    ) -> Vec<super::super::completions::message::Message> {
        use super::super::completions::message::{
            Message, SimpleContent, SystemMessage, UserMessage,
        };
        let system_len = if self.system_prompt.is_some() { 1 } else { 0 };
        let prefix_len = if self.prefix_content.is_some() { 1 } else { 0 };
        let suffix_len = if self.suffix_content.is_some() { 1 } else { 0 };
        let mut merged = Vec::with_capacity(
            system_len + prefix_len + messages.len() + suffix_len,
        );
        if let Some(system_prompt) = &self.system_prompt {
            merged.push(Message::System(SystemMessage {
                content: SimpleContent::Text(system_prompt.clone()),
                name: None,
            }));
        }
        let mut prefix_inserted = self.prefix_content.is_none();
        for msg in messages {
            if !prefix_inserted {
                if !matches!(msg, Message::System(_) | Message::Developer(_)) {
                    merged.push(Message::User(UserMessage {
                        content: self.prefix_content.clone().unwrap(),
                        name: None,
                    }));
                    prefix_inserted = true;
                }
            }
            merged.push(msg);
        }
        if !prefix_inserted {
            merged.push(Message::User(UserMessage {
                content: self.prefix_content.clone().unwrap(),
                name: None,
            }));
        }
        if let Some(suffix_content) = &self.suffix_content {
            merged.push(Message::User(UserMessage {
                content: suffix_content.clone(),
                name: None,
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

/// A validated Gemini Agent with its computed content-addressed ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.gemini.Agent")]
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
