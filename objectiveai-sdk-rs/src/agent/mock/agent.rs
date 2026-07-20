//! Mock Agent types and validation logic.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

/// The base configuration for a Mock Agent (without computed ID).
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
#[schemars(rename = "agent.mock.AgentBase")]
pub struct AgentBase {
    /// The upstream provider marker.
    pub upstream: super::Upstream,

    /// The output mode for vector completions. Ignored for agent completions.
    pub output_mode: super::OutputMode,

    /// Number of top log probabilities to return (2-20).
    ///
    /// **Vector completions only.** Ignored for agent completions.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub top_logprobs: Option<u64>,

    /// If true, the mock client will return an error instead of a response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<bool>,

    /// Probability (0-100) that the mock returns an error mid-stream.
    /// Requires `error` to be `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error_probability: Option<u8>,

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

    /// Client-side ObjectiveAI MCP surface the calling client is
    /// expected to expose locally back to the API (objectiveai
    /// built-in, plus specific plugins / tools by owner+name+version).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub client_objectiveai_mcp: Option<super::super::ClientObjectiveaiMcp>,

    /// Expose the built-in `objectiveai-mcp` to this agent. Canonical
    /// form keeps only `Some(true)` — `prepare` drops `false` / `None`
    /// (unspecified and explicitly-off hash identically to absent).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub objectiveai_mcp: Option<bool>,

    /// Plugins this agent uses — each IS one MCP server (the
    /// next-iteration plugin shape; see [`super::super::plugin`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub plugins: Vec<super::super::Plugin>,

    /// Deterministic-script override. When `Some`, the mock agent
    /// emits each [`super::Call`] as its own assistant turn —
    /// `tool_calls` first, then `content` — in array order. Each
    /// subsequent turn inspects the continuation to count how many
    /// `Call`s have already been satisfied (assistant message with
    /// exactly that `Call`'s `tool_calls` (by name+arguments) and
    /// `content`); the next un-matched `Call` is what that turn
    /// emits. Once every `Call` has been satisfied in the
    /// continuation, the mock falls through to its normal
    /// dispatcher. Pure addition — agents without `calls` are
    /// unaffected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub calls: Option<Vec<super::Call>>,
}

impl AgentBase {
    /// Normalizes the configuration for deterministic ID computation.
    pub fn prepare(&mut self) {
        self.top_logprobs = match self.top_logprobs {
            Some(0) | Some(1) => None,
            other => other,
        };
        if self.error == Some(true) && self.error_probability == Some(0) {
            self.error = None;
            self.error_probability = None;
        }
        if self.error == Some(false) {
            self.error = None;
        }
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
        self.client_objectiveai_mcp = match self.client_objectiveai_mcp.take() {
            Some(cm) => super::super::client_objectiveai_mcp::prepare(cm),
            None => None,
        };
        self.objectiveai_mcp = match self.objectiveai_mcp {
            Some(true) => Some(true),
            _ => None,
        };
        self.plugins =
            super::super::plugin::prepare(std::mem::take(&mut self.plugins));
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(top_logprobs) = self.top_logprobs
            && top_logprobs > 20
        {
            return Err("`top_logprobs` must be at most 20".to_string());
        }
        if let Some(mcp_servers) = &self.mcp_servers {
            super::super::mcp::mcp_servers::validate(mcp_servers)?;
        }
        if let Some(laboratories) = &self.laboratories {
            super::super::laboratory::laboratories::validate(laboratories)?;
        }
        if let Some(cm) = &self.client_objectiveai_mcp {
            super::super::client_objectiveai_mcp::validate(cm)?;
        }
        super::super::plugin::validate(&self.plugins)?;
        if let Some(p) = self.error_probability {
            if p > 100 {
                return Err(
                    "`error_probability` must be at most 100".to_string()
                );
            }
            if self.error != Some(true) {
                return Err("`error_probability` requires `error` to be true"
                    .to_string());
            }
        }
        Ok(())
    }

    /// Returns the messages as-is.
    pub fn merged_messages(
        &self,
        messages: Vec<super::super::completions::message::Message>,
    ) -> Vec<super::super::completions::message::Message> {
        messages
    }

    /// Computes the deterministic content-addressed ID.
    pub fn id(&self) -> String {
        let mut hasher = XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(self).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }

    pub const fn model() -> &'static str {
        "mock"
    }
}

/// A validated Mock Agent with its computed content-addressed ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.mock.Agent")]
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
