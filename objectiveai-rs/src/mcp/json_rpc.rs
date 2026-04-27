//! JSON-RPC 2.0 envelope types used by the MCP transport.

/// JSON-RPC 2.0 inbound request.
#[derive(Debug, serde::Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response envelope.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse<T> {
    Success {
        jsonrpc: String,
        id: serde_json::Value,
        result: T,
    },
    Error {
        jsonrpc: String,
        id: serde_json::Value,
        error: JsonRpcError,
    },
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 notification (no `id` field).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}
