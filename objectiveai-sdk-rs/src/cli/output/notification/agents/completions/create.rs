use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Final assistant message text returned by `agents completions create`.
///
/// Wire: `{"type":"notification","content":"...text..."}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.agents.completions.Content")]
pub struct Content {
    pub content: String,
}
