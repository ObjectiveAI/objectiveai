//! Typed server → client JSON-RPC notifications — the frames a server
//! pushes over the connection's standing SSE stream (the streamable
//! HTTP transport's GET channel). Derive-only, method-tagged via
//! single-variant marker enums, same shape as
//! [`super::JsonRpcRequest`] (see that module's docs for how the
//! untagged + marker + fallback pattern works).
//!
//! Plugin servers emit the `cli_request` extension frame as an rmcp
//! `CustomNotification` — on the wire that is a plain
//! `{"jsonrpc":"2.0","method":…,"params":…}` frame, indistinguishable
//! from a spec notification except by method.
//!
//! The SSE listener ignores [`JsonRpcServerNotification::Fallback`]
//! frames; a `cli_request` with unusable params lands there with the
//! KNOWN method marker, so consumers that want to be loud about our
//! own extension misbehaving can match
//! `Fallback { method: ServerNotificationMethod::CliRequest(_), .. }`.

use schemars::JsonSchema;

use super::request::method_marker;

method_marker!(
    /// The literal `"notifications/tools/list_changed"`.
    ToolsListChangedMethod, ToolsListChanged, "notifications/tools/list_changed"
);
method_marker!(
    /// The literal `"notifications/resources/list_changed"`.
    ResourcesListChangedMethod,
    ResourcesListChanged,
    "notifications/resources/list_changed"
);
method_marker!(
    /// The literal `"notifications/objectiveai/cli_request"` — the
    /// command-execution extension.
    CliRequestMethod, CliRequest, "notifications/objectiveai/cli_request"
);

/// The method of a [`JsonRpcServerNotification::Fallback`] frame:
/// every known server-notification method marker, then a `String`
/// catch-all.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.ServerNotificationMethod")]
pub enum ServerNotificationMethod {
    ToolsListChanged(ToolsListChangedMethod),
    ResourcesListChanged(ResourcesListChangedMethod),
    CliRequest(CliRequestMethod),
    Other(String),
}

/// A typed server → client JSON-RPC notification frame.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.JsonRpcServerNotification")]
pub enum JsonRpcServerNotification {
    /// `notifications/tools/list_changed`. Params are ignored (and
    /// tolerated — rmcp may attach `_meta`).
    ToolsListChanged {
        jsonrpc: String,
        method: ToolsListChangedMethod,
    },
    /// `notifications/resources/list_changed`. Params are ignored.
    ResourcesListChanged {
        jsonrpc: String,
        method: ResourcesListChangedMethod,
    },
    /// `notifications/objectiveai/cli_request` — the command-execution
    /// extension: the server asks this client to run a CLI command and
    /// POST the result stream back.
    CliRequest {
        jsonrpc: String,
        method: CliRequestMethod,
        params: CliRequestParams,
    },
    /// Total catch-all — unknown method, or a known method with
    /// unusable params. The listener ignores it.
    Fallback {
        jsonrpc: String,
        method: ServerNotificationMethod,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        params: Option<serde_json::Value>,
    },
}

impl JsonRpcServerNotification {
    pub fn tools_list_changed() -> Self {
        JsonRpcServerNotification::ToolsListChanged {
            jsonrpc: "2.0".to_string(),
            method: ToolsListChangedMethod::ToolsListChanged,
        }
    }

    pub fn resources_list_changed() -> Self {
        JsonRpcServerNotification::ResourcesListChanged {
            jsonrpc: "2.0".to_string(),
            method: ResourcesListChangedMethod::ResourcesListChanged,
        }
    }

    pub fn cli_request(params: CliRequestParams) -> Self {
        JsonRpcServerNotification::CliRequest {
            jsonrpc: "2.0".to_string(),
            method: CliRequestMethod::CliRequest,
            params,
        }
    }
}

/// Params of a [`JsonRpcServerNotification::CliRequest`] frame — the
/// command-execution extension's request envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[schemars(rename = "mcp.CliRequestParams")]
pub struct CliRequestParams {
    /// Server-minted correlation id. Every response item this client
    /// POSTs back for this run carries the same id, and the terminal
    /// frame closes it. Ids are scoped to the MCP session. (This is
    /// the extension's own correlation id, a layer above JSON-RPC —
    /// notifications have no [`super::RequestId`].)
    pub id: String,
    /// The CLI command to run.
    pub request: crate::cli::command::Request,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal typed CLI request (the `update` leaf — no
    /// subcommand-specific payload beyond the shared envelope).
    fn update_request() -> crate::cli::command::Request {
        crate::cli::command::Request::Update(
            crate::cli::command::update::Request {
                path_type: crate::cli::command::update::Path::Update,
                base: crate::cli::command::RequestBase {
                    jq: None,
                    python: None,
                    timeout_seconds: None,
                    max_tokens: None,
                },
            },
        )
    }

    /// The exact frame shape rmcp's `CustomNotification` emits (raw
    /// `params` passthrough, no `_meta`) deserializes into the typed
    /// `CliRequest` variant.
    #[test]
    fn cli_request_deserializes_from_rmcp_custom_wire_shape() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/objectiveai/cli_request",
            "params": {
                "id": "42",
                "request": serde_json::to_value(update_request()).unwrap(),
            },
        });
        let parsed: JsonRpcServerNotification =
            serde_json::from_str(&frame.to_string()).unwrap();
        let JsonRpcServerNotification::CliRequest { params, .. } = parsed
        else {
            panic!("expected CliRequest, got {parsed:?}");
        };
        assert_eq!(params.id, "42");
        assert_eq!(
            serde_json::to_value(&params.request).unwrap(),
            frame["params"]["request"],
        );
    }

    /// A `cli_request` frame without params lands on Fallback with the
    /// KNOWN method marker — ignorable by the listener, loud for
    /// anyone who cares to match it.
    #[test]
    fn cli_request_without_params_is_fallback() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/objectiveai/cli_request",
        });
        let parsed: JsonRpcServerNotification =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcServerNotification::Fallback {
                method: ServerNotificationMethod::CliRequest(_),
                ..
            },
        ));
    }

    /// list_changed frames map to their typed variants with or without
    /// a params object (rmcp may attach `_meta`).
    #[test]
    fn list_changed_frames_ignore_params() {
        for (method, want_tools) in [
            ("notifications/tools/list_changed", true),
            ("notifications/resources/list_changed", false),
        ] {
            for params in [None, Some(serde_json::json!({"_meta": {"x": 1}}))]
            {
                let mut frame = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": method,
                });
                if let Some(p) = params {
                    frame["params"] = p;
                }
                let parsed: JsonRpcServerNotification =
                    serde_json::from_str(&frame.to_string()).unwrap();
                match (want_tools, &parsed) {
                    (
                        true,
                        JsonRpcServerNotification::ToolsListChanged { .. },
                    ) => {}
                    (
                        false,
                        JsonRpcServerNotification::ResourcesListChanged {
                            ..
                        },
                    ) => {}
                    _ => panic!("wrong variant for {method}: {parsed:?}"),
                }
            }
        }
    }

    /// Unknown methods land on Fallback with `Other`.
    #[test]
    fn unknown_method_is_fallback_other() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/prompts/list_changed",
            "params": {"k": "v"},
        });
        let parsed: JsonRpcServerNotification =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcServerNotification::Fallback {
                method: ServerNotificationMethod::Other(method),
                ..
            } if method == "notifications/prompts/list_changed",
        ));
    }

    /// Serialize → deserialize round-trips, and serialization emits
    /// the full JSON-RPC frame.
    #[test]
    fn round_trip() {
        let cli = JsonRpcServerNotification::cli_request(CliRequestParams {
            id: "7".to_string(),
            request: update_request(),
        });
        let value = serde_json::to_value(&cli).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(
            value["method"],
            "notifications/objectiveai/cli_request"
        );
        assert_eq!(value["params"]["id"], "7");
        let back: JsonRpcServerNotification =
            serde_json::from_str(&value.to_string()).unwrap();
        assert!(matches!(
            back,
            JsonRpcServerNotification::CliRequest { params, .. }
                if params.id == "7",
        ));

        let tools = serde_json::to_value(
            JsonRpcServerNotification::tools_list_changed(),
        )
        .unwrap();
        assert_eq!(
            tools,
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed",
            }),
        );
        assert!(matches!(
            serde_json::from_str::<JsonRpcServerNotification>(
                &tools.to_string()
            )
            .unwrap(),
            JsonRpcServerNotification::ToolsListChanged { .. },
        ));
    }
}
