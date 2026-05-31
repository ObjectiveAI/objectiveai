use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire shape: `{"type":"notification","value":{"kind":"me","agent_id":"..."}}`.
/// Emitted by `objectiveai agents me`. The configured self agent id,
/// read from `Config.agent_id` — sourced from `OBJECTIVEAI_AGENT_ID`
/// for direct CLI, or from the `X-OBJECTIVEAI-AGENT-ID` header when
/// running under the MCP server (which defaults to `"MCP"` when the
/// header is absent).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Me")]
pub struct Me {
    pub agent_id: String,
}
