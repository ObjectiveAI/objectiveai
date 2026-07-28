//! Per-`response_id` MCP dispatch: route MCP ops to the API over the
//! agent-completion streaming WebSocket, in-process.
//!
//! `agents spawn` and `functions execute` each open one WS to the API
//! and obtain a [`Notifier`]. As soon as a chunk surfaces a new
//! agent-completion `response_id`, the conduit registers
//! `(response_id, notifier)` in the resident daemon's
//! `mcp_notifiers` map (on `GlobalContext`'s resident hubs). The
//! `agents mcp {tools,resources,servers} *` commands — which run
//! in-process in the same daemon — look the notifier up by
//! `response_id` and call it directly via [`call_notifier`]. There is
//! no socket: producer and consumer share one process.
//!
//! Contract: exactly one op -> one [`SocketResponse`]. The request is a
//! [`SocketRequest`] (internally tagged by the MCP method `path`); the
//! reply is a `{type, value}` [`SocketResponse`]. (The names keep the
//! `Socket*` prefix for continuity with the former wire shape; the
//! transport is now a direct call.)

use objectiveai_sdk::Notifier;
use objectiveai_sdk::client_objectiveai_mcp::server_response::JsonRpcResult;
use objectiveai_sdk::mcp;
use serde::{Deserialize, Serialize};

use crate::context::{GlobalContext, ScopedContext};

/// JSON-RPC server-error code reported for failures the proxy never
/// produced — a missing notifier or an unreachable resident daemon.
const SOCKET_ERR_CODE: i64 = -32099;

/// One MCP op addressed to a `response_id`. Internally tagged by the MCP
/// method `path` (`tools/list`, `tools/call`, `resources/list`,
/// `resources/read`, `servers/list`); the op's params are flattened
/// alongside. The `response_id` travels separately (the command's
/// `--response-id`), not on this shape.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "path")]
pub enum SocketRequest {
    #[serde(rename = "tools/list")]
    ListTools {
        #[serde(flatten)]
        params: mcp::tool::ListToolsRequest,
        /// Restrict the listing to the server with this name (routing
        /// prefix). `None` lists every server.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "tools/call")]
    CallTool(mcp::tool::CallToolRequestParams),
    #[serde(rename = "resources/list")]
    ListResources {
        #[serde(flatten)]
        params: mcp::resource::ListResourcesRequest,
        /// Restrict the listing to the server with this name (routing
        /// prefix). `None` lists every server.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "resources/read")]
    ReadResource(mcp::resource::ReadResourceRequestParams),
    /// List the proxy's connected upstream MCP servers + metadata. A
    /// proxy-local aggregate with no MCP params — a unit variant.
    #[serde(rename = "servers/list")]
    ListServers,
}

/// The MCP-op result: `{type, value}`. On success `value` is the
/// embedded MCP result as-is (one of the four result types); on failure
/// `value` is `{code, message}`. No transport envelope, no JSON-RPC
/// `data`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SocketResponse<R> {
    Ok(R),
    Err(McpError),
}

/// MCP error object embedded in [`SocketResponse::Err`].
#[derive(Debug, Serialize, Deserialize)]
pub struct McpError {
    pub code: i64,
    pub message: String,
}

/// Run one MCP op against the `response_id`'s registered [`Notifier`],
/// in-process. Looks the notifier up in the resident daemon's
/// `mcp_notifiers` map (`GlobalContext`'s resident hubs), dispatches the op,
/// and decodes the rendered result as `SocketResponse<R>` — a JSON
/// round-trip identical to what the former per-response socket did, so
/// `R` decodes exactly as before.
///
/// A missing notifier (no listener for this `response_id`) or a
/// non-resident-daemon context surfaces as `io::Error` — the same shape
/// the former connect failure produced.
pub async fn call_notifier<R: serde::de::DeserializeOwned>(
    global: &GlobalContext, _scoped: &ScopedContext,
    response_id: &str,
    request: &SocketRequest,
) -> std::io::Result<SocketResponse<R>> {
    let Some(hubs) = global.resident_hubs() else {
        return Err(std::io::Error::other(
            "mcp notifier lookup requires the resident daemon",
        ));
    };
    let Some(notifier) = hubs.mcp_notifiers.get(response_id).map(|n| n.1.clone()) else {
        return Err(std::io::Error::other(format!(
            "no mcp listener for response {response_id}"
        )));
    };
    let reply = dispatch(&notifier, response_id, request).await;
    serde_json::from_str::<SocketResponse<R>>(&reply)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Dispatch one op to the notifier and render its outcome to the
/// `{type, value}` reply text. The op params are cloned out of the
/// borrowed request (the notifier methods take owned params).
async fn dispatch(notifier: &Notifier, response_id: &str, request: &SocketRequest) -> String {
    let response_id = response_id.to_string();
    match request {
        SocketRequest::ListTools { params, name } => {
            render(notifier.list_tools(response_id, name.clone(), params.clone()).await)
        }
        SocketRequest::CallTool(params) => {
            render(notifier.call_tool(response_id, params.clone()).await)
        }
        SocketRequest::ListResources { params, name } => {
            render(notifier.list_resources(response_id, name.clone(), params.clone()).await)
        }
        SocketRequest::ReadResource(params) => {
            render(notifier.read_resource(response_id, params.clone()).await)
        }
        SocketRequest::ListServers => render(notifier.list_servers(response_id).await),
    }
}

/// Map a notifier MCP-op outcome into the `{type, value}` reply text.
/// The proxy's `JsonRpcResult::Err` and a transport failure both render
/// as `type:"err"`; only the code source differs (the proxy's own code
/// vs. [`SOCKET_ERR_CODE`]).
fn render<R, E>(result: Result<JsonRpcResult<R>, E>) -> String
where
    R: Serialize,
    E: std::fmt::Display,
{
    let response = match result {
        Ok(JsonRpcResult::Ok { result }) => SocketResponse::Ok(result),
        Ok(JsonRpcResult::Err { code, message, .. }) => {
            SocketResponse::Err(McpError { code, message })
        }
        Err(e) => SocketResponse::Err(McpError {
            code: SOCKET_ERR_CODE,
            message: e.to_string(),
        }),
    };
    serde_json::to_string(&response)
        .unwrap_or_else(|_| err_body(SOCKET_ERR_CODE, "serialize error"))
}

/// Last-resort hand-built err body for the (near-impossible) case that
/// serializing a [`SocketResponse`] itself fails.
fn err_body(code: i64, message: &str) -> String {
    format!(r#"{{"type":"err","value":{{"code":{code},"message":"{message}"}}}}"#)
}
