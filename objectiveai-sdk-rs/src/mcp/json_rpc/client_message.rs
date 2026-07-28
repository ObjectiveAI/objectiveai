//! The top-level client → server message union — one parse for the
//! MCP endpoint's inbound POST body.

use schemars::JsonSchema;

use super::{JsonRpcClientNotification, JsonRpcRequest};

/// Any JSON-RPC message a client POSTs to the MCP endpoint: a request
/// (has an `id`) or a notification (no `id`). Untagged, `Request`
/// first: every request variant requires an `id` field, so an id-less
/// frame falls through to the notification arm — the JSON-RPC kind
/// discrimination happens structurally, in one parse.
///
/// An explicit `"id": null` frame lands on the notification arm too —
/// MCP forbids null ids, so a null-id frame is by definition not a
/// correlatable request. (Notification struct variants ignore the
/// unknown `id` key.)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.JsonRpcClientMessage")]
pub enum JsonRpcClientMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcClientNotification),
}

#[cfg(test)]
mod tests {
    use super::super::{ClientNotificationMethod, ClientRequestMethod};
    use super::*;

    #[test]
    fn id_present_is_request() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "ping",
        });
        let parsed: JsonRpcClientMessage =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcClientMessage::Request(JsonRpcRequest::Ping { .. }),
        ));
    }

    #[test]
    fn id_absent_is_notification() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        });
        let parsed: JsonRpcClientMessage =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcClientMessage::Notification(
                JsonRpcClientNotification::Initialized { .. },
            ),
        ));
    }

    /// MCP forbids null ids — an explicit `"id": null` frame is not a
    /// correlatable request, so it lands on the notification arm (as
    /// an ignorable fallback: "ping" is not a notification method).
    #[test]
    fn null_id_is_notification() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": null,
            "method": "ping",
        });
        let parsed: JsonRpcClientMessage =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcClientMessage::Notification(
                JsonRpcClientNotification::Fallback {
                    method: ClientNotificationMethod::Other(method),
                    ..
                },
            ) if method == "ping",
        ));
    }

    /// Malformed cancelled params still parse (as an ignorable
    /// notification) — the 202 contract survives the union.
    #[test]
    fn bad_cancelled_params_is_ignorable_notification() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {"requestId": {"bad": true}},
        });
        let parsed: JsonRpcClientMessage =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcClientMessage::Notification(
                JsonRpcClientNotification::Fallback {
                    method: ClientNotificationMethod::Cancelled(_),
                    ..
                },
            ),
        ));
    }

    /// A request with a known method and bad params keeps its id — it
    /// parses as `Request(Fallback)` with the known method marker, so
    /// the receiver can answer -32602 WITH the id.
    #[test]
    fn bad_params_request_keeps_id() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {"no_name": true},
        });
        let parsed: JsonRpcClientMessage =
            serde_json::from_str(&frame.to_string()).unwrap();
        let JsonRpcClientMessage::Request(JsonRpcRequest::Fallback {
            id,
            method: ClientRequestMethod::CallTool(_),
            ..
        }) = parsed
        else {
            panic!("expected Request(Fallback with CallTool marker)");
        };
        assert_eq!(id, super::super::RequestId::Number(6.into()));
    }
}
