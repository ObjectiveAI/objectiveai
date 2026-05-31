use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Markdown instructions rendered to the JSONL stream by
/// `objectiveai plugins install` and `objectiveai tools install`
/// — text the caller is meant to read before continuing.
///
/// Wire: `{"type":"notification","value":{"kind":"instructions","instructions":"…markdown…"}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Instructions")]
pub struct Instructions {
    pub instructions: String,
}
