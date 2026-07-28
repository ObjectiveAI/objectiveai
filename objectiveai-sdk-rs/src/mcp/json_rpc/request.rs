//! The JSON-RPC request frame — typed, method-tagged, derive-only.
//!
//! Every variant carries the FULL frame (`jsonrpc`, `id`, `method`,
//! `params`) — there is no envelope type. The `method` field of each
//! typed variant is a single-variant enum that only (de)serializes its
//! one exact method string, so `#[serde(untagged)]` trying variants in
//! order IS the method dispatch: a frame with a different method fails
//! that variant's `method` field and falls through.
//!
//! The LAST variant, [`JsonRpcRequest::Fallback`], is total over
//! well-formed id-bearing frames: `method` is [`ClientRequestMethod`]
//! (the union of every known method marker plus a `String` catch-all)
//! and `params` is raw JSON. A frame lands there in exactly two cases,
//! distinguishable by matching the fallback's `method`:
//!
//! - a KNOWN method marker → the method exists but its params were
//!   missing/malformed (receivers answer `-32602` WITH the id);
//! - [`ClientRequestMethod::Other`] → unknown method (`-32601`).
//!
//! Either way the id survives, so no valid frame ever loses its id to
//! a parse failure.

use schemars::JsonSchema;

use super::RequestId;

macro_rules! method_marker {
    ($(#[$doc:meta])* $name:ident, $variant:ident, $method:literal) => {
        $(#[$doc])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            serde::Serialize,
            serde::Deserialize,
            JsonSchema,
        )]
        pub enum $name {
            #[serde(rename = $method)]
            $variant,
        }

        impl $name {
            pub const METHOD: &'static str = $method;
        }
    };
}
pub(super) use method_marker;

method_marker!(
    /// The literal `"initialize"`.
    InitializeMethod, Initialize, "initialize"
);
method_marker!(
    /// The literal `"ping"`.
    PingMethod, Ping, "ping"
);
method_marker!(
    /// The literal `"tools/list"`.
    ListToolsMethod, ListTools, "tools/list"
);
method_marker!(
    /// The literal `"tools/call"`.
    CallToolMethod, CallTool, "tools/call"
);
method_marker!(
    /// The literal `"resources/list"`.
    ListResourcesMethod, ListResources, "resources/list"
);
method_marker!(
    /// The literal `"resources/read"`.
    ReadResourceMethod, ReadResource, "resources/read"
);

/// The method of a [`JsonRpcRequest::Fallback`] frame: every known
/// request method marker, then a `String` catch-all. Matching a known
/// marker here means "known method, unusable params" (→ `-32602`);
/// [`ClientRequestMethod::Other`] means "unknown method" (→ `-32601`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.ClientRequestMethod")]
pub enum ClientRequestMethod {
    Initialize(InitializeMethod),
    Ping(PingMethod),
    ListTools(ListToolsMethod),
    CallTool(CallToolMethod),
    ListResources(ListResourcesMethod),
    ReadResource(ReadResourceMethod),
    Other(String),
}

impl ClientRequestMethod {
    /// The wire method string.
    pub fn as_str(&self) -> &str {
        match self {
            ClientRequestMethod::Initialize(_) => InitializeMethod::METHOD,
            ClientRequestMethod::Ping(_) => PingMethod::METHOD,
            ClientRequestMethod::ListTools(_) => ListToolsMethod::METHOD,
            ClientRequestMethod::CallTool(_) => CallToolMethod::METHOD,
            ClientRequestMethod::ListResources(_) => {
                ListResourcesMethod::METHOD
            }
            ClientRequestMethod::ReadResource(_) => {
                ReadResourceMethod::METHOD
            }
            ClientRequestMethod::Other(method) => method,
        }
    }
}

/// A typed client → server JSON-RPC request frame. See the module docs
/// for the dispatch rules; use the constructor methods to build frames
/// without spelling out the marker fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "mcp.JsonRpcRequest")]
pub enum JsonRpcRequest {
    Initialize {
        jsonrpc: String,
        id: RequestId,
        method: InitializeMethod,
        params: InitializeRequestParams,
    },
    /// Params are ignored (and tolerated) per spec.
    Ping {
        jsonrpc: String,
        id: RequestId,
        method: PingMethod,
    },
    ListTools {
        jsonrpc: String,
        id: RequestId,
        method: ListToolsMethod,
        /// Absent params are legal — an absent cursor.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        params: Option<crate::mcp::tool::ListToolsRequest>,
    },
    CallTool {
        jsonrpc: String,
        id: RequestId,
        method: CallToolMethod,
        params: crate::mcp::tool::CallToolRequestParams,
    },
    ListResources {
        jsonrpc: String,
        id: RequestId,
        method: ListResourcesMethod,
        /// Absent params are legal — an absent cursor.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        params: Option<crate::mcp::resource::ListResourcesRequest>,
    },
    ReadResource {
        jsonrpc: String,
        id: RequestId,
        method: ReadResourceMethod,
        params: crate::mcp::resource::ReadResourceRequestParams,
    },
    /// Total catch-all — known method with unusable params, or unknown
    /// method (see [`ClientRequestMethod`]). `params` is raw JSON here
    /// by definition: a frame only lands on this variant because its
    /// params could NOT be given a type.
    Fallback {
        jsonrpc: String,
        id: RequestId,
        method: ClientRequestMethod,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        params: Option<serde_json::Value>,
    },
}

impl JsonRpcRequest {
    pub fn initialize(id: RequestId, params: InitializeRequestParams) -> Self {
        JsonRpcRequest::Initialize {
            jsonrpc: "2.0".to_string(),
            id,
            method: InitializeMethod::Initialize,
            params,
        }
    }

    pub fn ping(id: RequestId) -> Self {
        JsonRpcRequest::Ping {
            jsonrpc: "2.0".to_string(),
            id,
            method: PingMethod::Ping,
        }
    }

    pub fn list_tools(
        id: RequestId,
        params: crate::mcp::tool::ListToolsRequest,
    ) -> Self {
        JsonRpcRequest::ListTools {
            jsonrpc: "2.0".to_string(),
            id,
            method: ListToolsMethod::ListTools,
            params: Some(params),
        }
    }

    pub fn call_tool(
        id: RequestId,
        params: crate::mcp::tool::CallToolRequestParams,
    ) -> Self {
        JsonRpcRequest::CallTool {
            jsonrpc: "2.0".to_string(),
            id,
            method: CallToolMethod::CallTool,
            params,
        }
    }

    pub fn list_resources(
        id: RequestId,
        params: crate::mcp::resource::ListResourcesRequest,
    ) -> Self {
        JsonRpcRequest::ListResources {
            jsonrpc: "2.0".to_string(),
            id,
            method: ListResourcesMethod::ListResources,
            params: Some(params),
        }
    }

    pub fn read_resource(
        id: RequestId,
        params: crate::mcp::resource::ReadResourceRequestParams,
    ) -> Self {
        JsonRpcRequest::ReadResource {
            jsonrpc: "2.0".to_string(),
            id,
            method: ReadResourceMethod::ReadResource,
            params,
        }
    }

    /// The frame's request id.
    pub fn id(&self) -> &RequestId {
        match self {
            JsonRpcRequest::Initialize { id, .. }
            | JsonRpcRequest::Ping { id, .. }
            | JsonRpcRequest::ListTools { id, .. }
            | JsonRpcRequest::CallTool { id, .. }
            | JsonRpcRequest::ListResources { id, .. }
            | JsonRpcRequest::ReadResource { id, .. }
            | JsonRpcRequest::Fallback { id, .. } => id,
        }
    }
}

/// Params of an `initialize` request.
///
/// Lenient on the receive side: only `protocolVersion` is required;
/// `capabilities` defaults and `clientInfo` may be absent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "mcp.InitializeRequestParams")]
pub struct InitializeRequestParams {
    pub protocol_version: String,
    #[serde(default)]
    pub capabilities: ClientCapabilities,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub client_info: Option<crate::mcp::initialize_result::Implementation>,
}

/// Capabilities a client declares at `initialize`. We declare none —
/// this serializes as `{}` — and the struct exists so the field is
/// typed and extensible rather than raw JSON.
#[derive(
    Debug, Clone, Default, serde::Serialize, serde::Deserialize, JsonSchema,
)]
#[schemars(rename = "mcp.ClientCapabilities")]
pub struct ClientCapabilities {}

#[cfg(test)]
mod tests {
    use super::*;

    fn number_id(n: u64) -> RequestId {
        RequestId::Number(n.into())
    }

    /// The typed initialize frame serializes to the exact legacy
    /// `json!` literal `client.rs` used to build by hand.
    #[test]
    fn initialize_matches_legacy_frame() {
        let request = JsonRpcRequest::initialize(
            number_id(1),
            InitializeRequestParams {
                protocol_version: "2025-06-18".to_string(),
                capabilities: ClientCapabilities::default(),
                client_info: Some(
                    crate::mcp::initialize_result::Implementation {
                        name: "objectiveai".to_string(),
                        title: None,
                        version: "1.2.3".to_string(),
                        website_url: None,
                        description: None,
                        icons: None,
                    },
                ),
            },
        );
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "objectiveai",
                        "version": "1.2.3",
                    },
                },
            }),
        );
    }

    /// Round-trip every constructor through the wire.
    #[test]
    fn round_trip() {
        let requests = [
            JsonRpcRequest::ping(number_id(2)),
            JsonRpcRequest::list_tools(
                RequestId::String("abc".to_string()),
                crate::mcp::tool::ListToolsRequest {
                    cursor: Some("c1".to_string()),
                },
            ),
            JsonRpcRequest::call_tool(
                number_id(3),
                crate::mcp::tool::CallToolRequestParams {
                    name: "echo".to_string(),
                    arguments: None,
                    _meta: None,
                    task: None,
                },
            ),
            JsonRpcRequest::read_resource(
                number_id(4),
                crate::mcp::resource::ReadResourceRequestParams {
                    uri: "file://x".to_string(),
                },
            ),
        ];
        for request in requests {
            let value = serde_json::to_value(&request).unwrap();
            let back: JsonRpcRequest =
                serde_json::from_str(&value.to_string()).unwrap();
            assert_eq!(back.id(), request.id());
            assert_eq!(serde_json::to_value(&back).unwrap(), value);
            assert!(
                !matches!(back, JsonRpcRequest::Fallback { .. }),
                "constructor round-trip must land on its typed variant",
            );
        }
    }

    /// JSON objects are unordered — params before method must parse
    /// identically.
    #[test]
    fn field_order_independent() {
        let frame = serde_json::json!({
            "params": {"uri": "file://x"},
            "method": "resources/read",
            "id": 9,
            "jsonrpc": "2.0",
        });
        let parsed: JsonRpcRequest =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcRequest::ReadResource { params, .. }
                if params.uri == "file://x",
        ));
    }

    /// Unknown methods land on Fallback with `Other` — the receiver
    /// answers -32601 with the id.
    #[test]
    fn unknown_method_is_fallback_other() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "prompts/list",
            "params": {"cursor": null},
        });
        let parsed: JsonRpcRequest =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcRequest::Fallback {
                method: ClientRequestMethod::Other(method),
                ..
            } if method == "prompts/list",
        ));
    }

    /// Known method + malformed params → Fallback with the KNOWN
    /// method marker and the id preserved — the receiver answers
    /// -32602 with the id.
    #[test]
    fn bad_params_is_fallback_with_known_method() {
        // tools/call with no params at all — missing required `name`.
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
        });
        let parsed: JsonRpcRequest =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert_eq!(parsed.id(), &number_id(6));
        assert!(matches!(
            parsed,
            JsonRpcRequest::Fallback {
                method: ClientRequestMethod::CallTool(_),
                ..
            },
        ));
    }

    /// tools/list with absent params parses onto its typed variant.
    #[test]
    fn list_tools_with_absent_params_parses() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/list",
        });
        let parsed: JsonRpcRequest =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcRequest::ListTools { params: None, .. },
        ));
    }

    /// initialize params are lenient: only protocolVersion required.
    #[test]
    fn initialize_with_only_protocol_version_parses() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "initialize",
            "params": {"protocolVersion": "2025-06-18"},
        });
        let parsed: JsonRpcRequest =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(
            parsed,
            JsonRpcRequest::Initialize { params, .. }
                if params.protocol_version == "2025-06-18"
                    && params.client_info.is_none(),
        ));
    }

    /// Ping tolerates (and ignores) params, per spec.
    #[test]
    fn ping_ignores_params() {
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "ping",
            "params": {"_meta": {"x": 1}},
        });
        let parsed: JsonRpcRequest =
            serde_json::from_str(&frame.to_string()).unwrap();
        assert!(matches!(parsed, JsonRpcRequest::Ping { .. }));
    }
}
