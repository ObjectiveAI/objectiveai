use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of a user-supplied jq filter applied to a config getter.
/// `jq` is the one explicit escape hatch for untyped JSON: the filter
/// can produce literally any shape, so the body is `serde_json::Value`.
/// Wire: `{"type":"notification","jq":<value>}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.JqResults")]
pub struct JqResults {
    pub jq: serde_json::Value,
}
