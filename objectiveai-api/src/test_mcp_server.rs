//! In-process MCP HTTP server for unit tests. Bound to `127.0.0.1:0` so
//! the OS picks a free port.
//!
//! Tests never connect to a server spawned here directly — that would
//! diverge from production. Instead they use this server as one of the
//! upstreams the in-process `objectiveai-mcp-proxy` fans out to. The
//! flow is identical to runtime: agent → proxy → upstream(s).

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use indexmap::IndexMap;
use rmcp::{
    RoleServer, ServerHandler,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::tool::ToolCallContext,
    model::{
        CallToolRequestParams, CallToolResult, Content, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool as RmcpTool,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
        session::local::LocalSessionManager,
    },
};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

/// A tool the test server exposes: the wire-level definition plus the
/// callback that runs when the tool is invoked.
pub struct TestTool {
    pub tool: objectiveai_sdk::mcp::tool::Tool,
    pub call: Arc<
        dyn Fn(Value) -> futures::future::BoxFuture<'static, Result<String, String>>
            + Send
            + Sync,
    >,
}

impl TestTool {
    /// A tool whose call always returns "ok". Sufficient for tests that
    /// only care about `list_tools()` output, not invocation.
    pub fn noop(tool: objectiveai_sdk::mcp::tool::Tool) -> Self {
        Self {
            tool,
            call: Arc::new(|_| async { Ok("ok".into()) }.boxed()),
        }
    }

    /// Build a noop `TestTool` from raw name/description/parameters,
    /// standing in for an upstream MCP tool definition.
    pub fn from_parts(
        name: &str,
        description: &str,
        parameters: indexmap::IndexMap<String, Value>,
    ) -> Self {
        Self::noop(objectiveai_sdk::mcp::tool::Tool {
            name: name.to_string(),
            title: None,
            description: Some(description.to_string()),
            icons: None,
            input_schema: objectiveai_sdk::mcp::tool::ToolSchemaObject {
                r#type: objectiveai_sdk::mcp::tool::ToolSchemaType::Object,
                properties: None,
                required: None,
                extra: parameters,
            },
            output_schema: None,
            annotations: None,
            execution: None,
            _meta: None,
        })
    }
}

/// Handle to a spawned in-process MCP server. Drop to tear down.
pub struct TestMcpServer {
    /// The server's MCP endpoint URL (`http://127.0.0.1:<port>/mcp`).
    pub url: String,
    _cancel: CancellationToken,
    _server_handle: tokio::task::AbortHandle,
}

#[derive(Clone)]
struct TestMcp {
    tool_router: ToolRouter<Self>,
    name: String,
}

impl TestMcp {
    fn new(name: String, tools: Vec<TestTool>) -> Self {
        let mut tool_router = ToolRouter::<Self>::new();

        for t in tools {
            let input_schema: serde_json::Map<String, Value> = {
                let mut m = serde_json::Map::new();
                m.insert("type".into(), Value::String("object".into()));
                if let Some(props) = t.tool.input_schema.properties {
                    m.insert(
                        "properties".into(),
                        Value::Object(props.into_iter().collect()),
                    );
                }
                if let Some(req) = t.tool.input_schema.required {
                    m.insert(
                        "required".into(),
                        Value::Array(req.into_iter().map(Value::String).collect()),
                    );
                }
                for (k, v) in t.tool.input_schema.extra {
                    m.insert(k, v);
                }
                m
            };

            // rmcp 1.7 marks `Tool` `#[non_exhaustive]` — build via Default
            // + field assignment. (output_schema/annotations/execution/
            // icons/meta default to None.)
            let mut tool_def = RmcpTool::default();
            tool_def.name = Cow::Owned(t.tool.name);
            tool_def.title = t.tool.title;
            tool_def.description = t.tool.description.map(Cow::Owned);
            tool_def.input_schema = Arc::new(input_schema);

            let call_fn = t.call.clone();
            tool_router.add_route(ToolRoute::new_dyn(
                tool_def,
                move |ctx: ToolCallContext<'_, TestMcp>| {
                    let call_fn = call_fn.clone();
                    let arguments = ctx
                        .arguments
                        .clone()
                        .map(Value::Object)
                        .unwrap_or(Value::Object(Default::default()));
                    async move {
                        let result = call_fn(arguments).await;
                        match result {
                            Ok(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
                            Err(text) => Ok(CallToolResult::error(vec![Content::text(text)])),
                        }
                    }
                    .boxed()
                },
            ));
        }

        Self { tool_router, name }
    }
}

// Manual `ServerHandler` (not `#[tool_handler]`): the router is built
// dynamically at runtime via `ToolRouter::new()` + `add_route`, so there is
// no `#[tool_router]`-generated `Self::tool_router()` for the macro to call.
// list_tools/call_tool delegate to the field directly.
impl ServerHandler for TestMcp {
    fn get_info(&self) -> ServerInfo {
        // rmcp 1.7 marks `ServerInfo`/`Implementation` `#[non_exhaustive]`.
        let mut server_info = rmcp::model::Implementation::default();
        server_info.name = self.name.clone();
        server_info.version = "0.0.0".into();
        let mut info = ServerInfo::default();
        info.instructions = Some("ObjectiveAI test MCP server".into());
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tool_router.list_all()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let tcc = ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }
}

/// Spawn an in-process rmcp HTTP MCP server on a random localhost port.
/// Returns once the listener is bound. The OS-chosen port is read from
/// the listener's local address.
///
/// The proxy keys upstream connections by the server's `serverInfo.name`
/// (returned via `initialize`) and prefixes every tool name with
/// `<server_name>_`. Tests should pick names that match the prefixes
/// they expect downstream.
pub async fn spawn(name: impl Into<String>, tools: Vec<TestTool>) -> TestMcpServer {
    let name = name.into();
    let cancel = CancellationToken::new();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1:0");
    let port = listener
        .local_addr()
        .expect("local_addr after bind")
        .port();

    let ct_child = cancel.child_token();
    let mcp = TestMcp::new(name, tools);
    let server_handle = tokio::spawn(async move {
        let service: StreamableHttpService<TestMcp, LocalSessionManager> =
            StreamableHttpService::new(
                move || Ok(mcp.clone()),
                Default::default(),
                {
                    // rmcp 1.7 marks `StreamableHttpServerConfig`
                    // `#[non_exhaustive]`.
                    let mut cfg = StreamableHttpServerConfig::default();
                    cfg.stateful_mode = true;
                    cfg.sse_keep_alive = None;
                    cfg.cancellation_token = ct_child;
                    cfg
                },
            );

        let router = axum::Router::new().fallback_service(service);
        axum::serve(listener, router).await.ok();
    })
    .abort_handle();

    TestMcpServer {
        url: format!("http://127.0.0.1:{port}/mcp"),
        _cancel: cancel,
        _server_handle: server_handle,
    }
}

impl Drop for TestMcpServer {
    fn drop(&mut self) {
        self._server_handle.abort();
    }
}

/// Bootstrap an isolated proxy via the production `ProxyFactory` and
/// return a `Connection` that points at it with the given upstream
/// servers wired up via `X-MCP-Servers`. Mirrors `client.rs:449-531`
/// exactly so tests run through the same code path agents use at runtime.
///
/// HTTP-only: tests reach external `TestMcpServer`s over `X-MCP-Servers`,
/// so the proxy needs no reverse channel — `boot(None, ...)` with a
/// throwaway queue delegate (nothing reads its queue on this path).
pub async fn connect_through_proxy(
    servers: &[&TestMcpServer],
) -> objectiveai_sdk::mcp::Connection {
    let proxy = crate::test_clients::proxy_spawner()
        .boot(
            None,
            std::sync::Arc::new(crate::agent::completions::ApiQueueDelegate::new()),
        )
        .await
        .expect("proxy bootstrap");

    let urls: Vec<String> = servers.iter().map(|s| s.url.clone()).collect();

    let mut headers = IndexMap::<String, String>::new();
    headers.insert(
        "X-MCP-Servers".into(),
        serde_json::to_string(&urls).expect("serialize X-MCP-Servers"),
    );
    // X-MCP-Headers is a per-URL `{url: {header: value}}` map. Empty
    // map → no per-upstream headers to apply.
    headers.insert("X-MCP-Headers".into(), "{}".into());

    // Reuse the process-wide MCP client singleton. Its underlying
    // reqwest::Client is built with `pool_max_idle_per_host(0)` so each
    // request opens a fresh connection on the calling test's runtime —
    // no shared connection-pool task that could be tied to a dropped
    // per-test runtime.
    crate::test_clients::mcp_client()
        .connect(proxy.url.clone(), None, Some(headers))
        .await
        .expect("connect through proxy")
}
