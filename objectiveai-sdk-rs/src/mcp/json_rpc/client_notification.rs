//! Typed client → server JSON-RPC notifications — derive-only,
//! method-tagged via single-variant marker enums, same shape as
//! [`super::JsonRpcRequest`] (see that module's docs for how the
//! untagged + marker + fallback pattern works).
//!
//! Per the streamable HTTP transport spec, a client delivers EVERY
//! JSON-RPC message as an HTTP POST to the server's one MCP endpoint;
//! a body without an `id` is a notification and is answered
//! `202 Accepted` with no body. Receivers ignore any notification they
//! can't use — which is exactly where a frame lands when it parses
//! onto [`JsonRpcClientNotification::Fallback`] (unknown method, or a
//! known method with unusable params).

use schemars::JsonSchema;

use super::request::method_marker;
use super::RequestId;

method_marker!(
    /// The literal `"notifications/initialized"`.
    InitializedMethod, Initialized, "notifications/initialized"
);
method_marker!(
    /// The literal `"notifications/cancelled"`.
    CancelledMethod, Cancelled, "notifications/cancelled"
);

/// The method of a [`JsonRpcClientNotification::Fallback`] frame:
/// every known client-notification method marker, then a `String`
/// catch-all. A known marker here means "known method, unusable
/// params"; [`ClientNotificationMethod::Other`] means "unknown
/// method". Receivers ignore both.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.ClientNotificationMethod")]
pub enum ClientNotificationMethod {
    Initialized(InitializedMethod),
    Cancelled(CancelledMethod),
    Other(String),
}

/// A typed client → server JSON-RPC notification frame.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.JsonRpcClientNotification")]
pub enum JsonRpcClientNotification {
    /// `notifications/initialized` — the client's post-`initialize`
    /// handshake completion. Params are ignored (and tolerated).
    Initialized {
        jsonrpc: String,
        method: InitializedMethod,
    },
    /// `notifications/cancelled` — the client is cancelling a
    /// previously-issued, still-in-flight request.
    Cancelled {
        jsonrpc: String,
        method: CancelledMethod,
        params: CancelledNotificationParams,
    },
    /// Total catch-all — unknown method, or a known method with
    /// unusable params. Receivers ignore it (still `202`).
    Fallback {
        jsonrpc: String,
        method: ClientNotificationMethod,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        params: Option<serde_json::Value>,
    },
}

impl JsonRpcClientNotification {
    pub fn initialized() -> Self {
        JsonRpcClientNotification::Initialized {
            jsonrpc: "2.0".to_string(),
            method: InitializedMethod::Initialized,
        }
    }

    pub fn cancelled(params: CancelledNotificationParams) -> Self {
        JsonRpcClientNotification::Cancelled {
            jsonrpc: "2.0".to_string(),
            method: CancelledMethod::Cancelled,
            params,
        }
    }
}

/// Params of a [`JsonRpcClientNotification::Cancelled`] frame.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "mcp.CancelledNotificationParams")]
pub struct CancelledNotificationParams {
    /// The `id` of the in-flight request being cancelled.
    pub request_id: RequestId,
    /// Optional human-readable reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialized_deserializes_and_ignores_params() {
        for frame in [
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
            }),
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {"_meta": {"x": 1}},
            }),
        ] {
            let parsed: JsonRpcClientNotification =
                serde_json::from_str(&frame.to_string()).unwrap();
            assert!(matches!(
                parsed,
                JsonRpcClientNotification::Initialized { .. },
            ));
        }
    }

    #[test]
    fn cancelled_deserializes_string_and_number_ids() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {"requestId": "abc", "reason": "user aborted"},
        });
        let parsed: JsonRpcClientNotification =
            serde_json::from_str(&frame.to_string()).unwrap();
        let JsonRpcClientNotification::Cancelled { params, .. } = parsed
        else {
            panic!("expected Cancelled");
        };
        assert_eq!(params.request_id, RequestId::String("abc".to_string()));
        assert_eq!(params.reason.as_deref(), Some("user aborted"));

        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {"requestId": 42},
        });
        let parsed: JsonRpcClientNotification =
            serde_json::from_str(&frame.to_string()).unwrap();
        let JsonRpcClientNotification::Cancelled { params, .. } = parsed
        else {
            panic!("expected Cancelled");
        };
        assert_eq!(params.request_id, RequestId::Number(42.into()));
        assert_eq!(params.reason, None);
    }

    /// Unusable cancelled params fall to Fallback with the KNOWN
    /// method marker (receivers 202 and ignore) — never a hard error.
    #[test]
    fn cancelled_with_bad_params_is_fallback() {
        for frame in [
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/cancelled",
            }),
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/cancelled",
                "params": {"requestId": {"bad": true}},
            }),
        ] {
            let parsed: JsonRpcClientNotification =
                serde_json::from_str(&frame.to_string()).unwrap();
            assert!(matches!(
                parsed,
                JsonRpcClientNotification::Fallback {
                    method: ClientNotificationMethod::Cancelled(_),
                    ..
                },
            ));
        }
    }

    #[test]
    fn unknown_method_is_fallback_other() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/roots/list_changed",
        });
        let parsed: JsonRpcClientNotification =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcClientNotification::Fallback {
                method: ClientNotificationMethod::Other(method),
                ..
            } if method == "notifications/roots/list_changed",
        ));
    }

    #[test]
    fn round_trip() {
        let cancelled = JsonRpcClientNotification::cancelled(
            CancelledNotificationParams {
                request_id: RequestId::Number(7.into()),
                reason: None,
            },
        );
        let value = serde_json::to_value(&cancelled).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/cancelled",
                "params": {"requestId": 7},
            }),
        );
        assert!(matches!(
            serde_json::from_str::<JsonRpcClientNotification>(
                &value.to_string()
            )
            .unwrap(),
            JsonRpcClientNotification::Cancelled { params, .. }
                if params.request_id == RequestId::Number(7.into()),
        ));

        let initialized = serde_json::to_value(
            JsonRpcClientNotification::initialized(),
        )
        .unwrap();
        assert_eq!(
            initialized,
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
            }),
        );
    }
}
