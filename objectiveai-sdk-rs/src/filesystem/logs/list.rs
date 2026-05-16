use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single entry in a log listing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.logs.ListItem")]
pub struct ListItem {
    /// The log ID (filename stem, e.g. `"fexec1-abc123-1712345678"`).
    pub id: String,
    /// Unix timestamp (seconds) when the log was created.
    pub created: u64,
}
