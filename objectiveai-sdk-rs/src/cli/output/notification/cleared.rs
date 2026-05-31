use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Number of log files cleared by `<scope> logs clear` (or the global
/// `logs clear`). Wire: `{"type":"notification","cleared":7}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Cleared")]
pub struct Cleared {
    pub cleared: u64,
}
