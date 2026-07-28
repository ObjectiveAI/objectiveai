//! The JSON-RPC error object.

use schemars::JsonSchema;

/// JSON-RPC 2.0 error object.
///
/// `data` stays [`serde_json::Value`] deliberately: the spec leaves the
/// error payload open-ended (one of the few genuinely arbitrary spots
/// on the wire).
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(rename = "mcp.JsonRpcError")]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub data: Option<serde_json::Value>,
}
