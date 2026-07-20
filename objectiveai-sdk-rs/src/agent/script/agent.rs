//! Script Agent types and validation logic.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

/// The base configuration for a Script Agent (without computed ID).
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.script.AgentBase")]
pub struct AgentBase {
    /// The upstream provider marker.
    pub upstream: super::Upstream,

    /// The output mode for vector completions. Ignored for agent completions.
    pub output_mode: super::OutputMode,

    /// The code this agent runs on the CLIENT — a required
    /// `type`-tagged enum, flattened so `type` + its payload (e.g.
    /// `python`) sit directly on the agent object.
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<super::Script>")]
    pub script: super::Script,

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
}

impl AgentBase {
    /// Normalizes the configuration for deterministic ID computation.
    /// The script code itself is never normalized — whitespace is
    /// significant.
    pub fn prepare(&mut self) {
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
        self.objectiveai_mcp = match self.objectiveai_mcp {
            Some(true) => Some(true),
            _ => None,
        };
        self.plugins =
            super::super::plugin::prepare(std::mem::take(&mut self.plugins));
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), String> {
        self.script.validate()?;
        if let Some(mcp_servers) = &self.mcp_servers {
            super::super::mcp::mcp_servers::validate(mcp_servers)?;
        }
        if let Some(laboratories) = &self.laboratories {
            super::super::laboratory::laboratories::validate(laboratories)?;
        }
        super::super::plugin::validate(&self.plugins)?;
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
        "script"
    }
}

/// A validated Script Agent with its computed content-addressed ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.script.Agent")]
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
