use std::borrow::Cow;
use std::sync::Arc;

use futures::FutureExt;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::{ToolRoute, ToolRouter},
    handler::server::tool::ToolCallContext,
    model::{
        CallToolRequestParams, CallToolResult, Content, ServerCapabilities, ServerInfo, Tool,
    },
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
        session::local::LocalSessionManager,
    },
};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use objectiveai::functions::inventions::InventionTool;

/// In-process MCP HTTP server that wraps a set of `InventionTool` callables.
/// Used by `agent/completions/client.rs` whenever a request supplies
/// invention tools — its URL is added to the per-agent proxy connection's
/// `X-MCP-Servers` list so every upstream that the proxy fans out to can
/// reach the invention tools just like any other MCP server.
pub struct InventionServer {
    port: u16,
    _cancel: CancellationToken,
    server_handle: tokio::task::AbortHandle,
}

#[derive(Clone)]
struct InventionMcp {
    tool_router: ToolRouter<Self>,
}

impl InventionMcp {
    fn new(tools: Vec<InventionTool>) -> Self {
        let mut tool_router = ToolRouter::<Self>::new();

        for t in tools {
            let input_schema: serde_json::Map<String, Value> = t.parameters.into_iter().collect();

            let tool_def = Tool {
                name: Cow::Owned(t.name.to_string()),
                title: None,
                description: Some(Cow::Owned(t.description.to_string())),
                input_schema: Arc::new(input_schema),
                output_schema: None,
                annotations: None,
                execution: None,
                icons: None,
                meta: None,
            };

            let call_fn = t.call.clone();
            tool_router.add_route(ToolRoute::new_dyn(
                tool_def,
                move |ctx: ToolCallContext<'_, InventionMcp>| {
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

        Self { tool_router }
    }
}

#[rmcp::tool_handler]
impl ServerHandler for InventionMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("ObjectiveAI invention tool server".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Separate function to prevent rmcp generics from inflating the caller.
#[inline(never)]
fn build_and_spawn_server(
    tools: Vec<InventionTool>,
    ct: CancellationToken,
) -> (tokio::sync::oneshot::Receiver<u16>, tokio::task::AbortHandle) {
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();
    let ct_child = ct.child_token();

    let handle = tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let _ = port_tx.send(port);

        let mcp = InventionMcp::new(tools);
        let service: StreamableHttpService<InventionMcp, LocalSessionManager> =
            StreamableHttpService::new(
                move || Ok(mcp.clone()),
                Default::default(),
                StreamableHttpServerConfig {
                    stateful_mode: true,
                    sse_keep_alive: None,
                    cancellation_token: ct_child,
                    ..Default::default()
                },
            );

        let router = axum::Router::new().fallback_service(service);
        axum::serve(listener, router).await.ok();
    })
    .abort_handle();

    (port_rx, handle)
}

impl InventionServer {
    pub async fn new(tools: Vec<InventionTool>) -> Self {
        let ct = CancellationToken::new();
        let (port_rx, server_handle) = build_and_spawn_server(tools, ct.clone());
        let port = port_rx.await.unwrap();

        Self {
            port,
            _cancel: ct,
            server_handle,
        }
    }

    /// Streamable-HTTP MCP endpoint URL (one entry to add to the proxy's
    /// `X-MCP-Servers` array).
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }
}

impl Drop for InventionServer {
    fn drop(&mut self) {
        self.server_handle.abort();
    }
}

#[cfg(test)]
#[path = "invention_server_tests.rs"]
mod tests;
