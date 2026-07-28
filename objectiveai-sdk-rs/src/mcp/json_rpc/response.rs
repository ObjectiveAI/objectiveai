//! The JSON-RPC response envelope.

use schemars::JsonSchema;

use super::RequestId;

/// JSON-RPC 2.0 response envelope.
///
/// Untagged: a success frame carries `result`, an error frame carries
/// `error` — the presence of one and absence of the other is the
/// discriminator, per spec.
///
/// Id typing is asymmetric by design:
/// - `Success.id` is a required [`RequestId`] — a success always
///   answers a specific request.
/// - `Error.id` is `Option<RequestId>` — a parse error answers no
///   identifiable request, and the spec mandates `"id": null` there.
///   `None` serializes as an EXPLICIT `null` (no skip attribute), and
///   an absent `id` (rmcp omits it per MCP 2025-11-25) deserializes to
///   `None`.
#[derive(Debug, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.JsonRpcResponse.{T}", bound = "T: JsonSchema")]
pub enum JsonRpcResponse<T> {
    Success {
        jsonrpc: String,
        id: RequestId,
        result: T,
    },
    Error {
        jsonrpc: String,
        #[serde(default)]
        id: Option<RequestId>,
        error: super::JsonRpcError,
    },
}

/// The empty JSON object `{}` — the spec's result for `ping`.
///
/// A braced zero-field struct, NOT a unit struct: unit structs
/// serialize as `null`, braced ones as `{}`.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(rename = "mcp.EmptyObject")]
pub struct EmptyObject {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_and_error_discriminate_with_typed_ids() {
        for id in [serde_json::json!(7), serde_json::json!("abc")] {
            let success = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"ok": true},
            });
            assert!(matches!(
                serde_json::from_value::<JsonRpcResponse<serde_json::Value>>(
                    success
                )
                .unwrap(),
                JsonRpcResponse::Success { .. },
            ));

            let error = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "nope"},
            });
            assert!(matches!(
                serde_json::from_value::<JsonRpcResponse<serde_json::Value>>(
                    error
                )
                .unwrap(),
                JsonRpcResponse::Error { id: Some(_), .. },
            ));
        }
    }

    /// rmcp omits the error id per MCP 2025-11-25 — absent parses to
    /// `None`.
    #[test]
    fn error_with_absent_id_parses_none() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "error": {"code": -32700, "message": "parse error"},
        });
        assert!(matches!(
            serde_json::from_value::<JsonRpcResponse<EmptyObject>>(frame)
                .unwrap(),
            JsonRpcResponse::Error { id: None, .. },
        ));
    }

    /// `None` serializes as a literal `"id": null` — the classic
    /// JSON-RPC parse-error shape the proxy emits.
    #[test]
    fn error_with_none_id_serializes_explicit_null() {
        let frame = JsonRpcResponse::<EmptyObject>::Error {
            jsonrpc: "2.0".to_string(),
            id: None,
            error: super::super::JsonRpcError {
                code: -32700,
                message: "parse error".to_string(),
                data: None,
            },
        };
        let value = serde_json::to_value(&frame).unwrap();
        assert!(value.get("id").is_some());
        assert_eq!(value["id"], serde_json::Value::Null);
    }

    /// Documents the deliberate tightening: a success frame must carry
    /// a string-or-number id; `null` (spec-forbidden) fails both arms.
    #[test]
    fn success_with_null_id_is_rejected() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": null,
            "result": {},
        });
        assert!(
            serde_json::from_value::<JsonRpcResponse<EmptyObject>>(frame)
                .is_err()
        );
    }

    #[test]
    fn empty_object_serializes_to_braces() {
        assert_eq!(
            serde_json::to_value(EmptyObject {}).unwrap(),
            serde_json::json!({}),
        );
    }
}
