use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire shape for `<scope> instructions get` (and the global
/// `instructions get`). The body is the instruction text the user is
/// meant to follow plus the generated instructions ID line.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.Instructions")]
pub struct Instructions {
    pub instructions: String,
}
