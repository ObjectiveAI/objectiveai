//! Test-fixture RMCP plugin that injects a failure at one chosen stage —
//! connect (initialize) / list-tools / call-tool — selected by the
//! `--fail-at <connect|list|call>` arg the conduit forwards from the
//! agent's `mcp_servers[].arguments`. Drives the cli MCP-failure tests.
//!
//! Unlike `test-mcp-plugin` (which uses the `#[tool_handler]` macro), this
//! one keeps `#[tool_router]` for a single happy-path tool `doit` but
//! implements `ServerHandler` MANUALLY so it can return an error from
//! `initialize` / `list_tools` / `call_tool`. `serverInfo.name` is
//! `failsvr`, so the tool surfaces through the proxy as `failsvr_doit`.
//!
//! Like `test-mcp-plugin`, it writes its PID to `OAI_TEST_MCP_PID_FILE`
//! and announces its bound URL as a `{"type":"mcp","url":...}` line before
//! serving, so the test's `Drop` guard can force-kill it.

use std::io::Write;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Implementation, InitializeRequestParams,
    InitializeResult, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{RoleServer, ServerHandler, tool, tool_router};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailMode {
    Connect,
    List,
    Call,
}

#[derive(Debug, Clone)]
struct FailMcp {
    fail: FailMode,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl FailMcp {
    fn new(fail: FailMode) -> Self {
        Self {
            fail,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(name = "doit", description = "no-op test tool")]
    async fn doit(&self) -> String {
        "ok".to_string()
    }
}

impl ServerHandler for FailMcp {
    fn get_info(&self) -> ServerInfo {
        // rmcp 1.7 marks `ServerInfo`/`Implementation` `#[non_exhaustive]`.
        let mut server_info = Implementation::default();
        server_info.name = "failsvr".into();
        server_info.version = env!("CARGO_PKG_VERSION").into();
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::V_2025_06_18;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info.instructions = None;
        info
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, rmcp::ErrorData> {
        if self.fail == FailMode::Connect {
            return Err(rmcp::ErrorData::internal_error(
                "injected connect failure",
                None,
            ));
        }
        // Replicate the default initialize so a non-connect-fail handshake
        // behaves normally.
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        if self.fail == FailMode::List {
            return Err(rmcp::ErrorData::internal_error(
                "injected list-tools failure",
                None,
            ));
        }
        Ok(ListToolsResult::with_all_items(self.tool_router.list_all()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        if self.fail == FailMode::Call {
            return Err(rmcp::ErrorData::internal_error(
                "injected call-tool failure",
                None,
            ));
        }
        let tcc = ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let ok = args.len() >= 4 && args[1] == "mcp" && args[3] == "begin";
    if !ok {
        eprintln!(
            "usage: test-mcp-plugin-failing mcp <server_name> begin --fail-at <connect|list|call>"
        );
        std::process::exit(2);
    }

    // `--fail-at <mode>` is forwarded by the conduit from the agent's
    // `mcp_servers[].arguments`. Default to `call`.
    let mut fail = FailMode::Call;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--fail-at" {
            if let Some(v) = it.next() {
                fail = match v.as_str() {
                    "connect" => FailMode::Connect,
                    "list" => FailMode::List,
                    "call" => FailMode::Call,
                    other => {
                        eprintln!("unknown --fail-at value: {other:?}");
                        std::process::exit(2);
                    }
                };
            }
        }
    }

    let service: StreamableHttpService<FailMcp, LocalSessionManager> =
        StreamableHttpService::new(move || Ok(FailMcp::new(fail)), Default::default(), {
            // rmcp 1.7 marks `StreamableHttpServerConfig` `#[non_exhaustive]`.
            let mut cfg = StreamableHttpServerConfig::default();
            cfg.stateful_mode = true;
            cfg.sse_keep_alive = None;
            cfg.cancellation_token = CancellationToken::new().child_token();
            cfg
        });
    let router = axum::Router::new().fallback_service(service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // PID handoff for the test's Drop-guard cleanup — write before
    // announcing the URL.
    if let Ok(path) = std::env::var("OAI_TEST_MCP_PID_FILE") {
        let _ = std::fs::write(&path, std::process::id().to_string());
    }

    let url = format!("http://{}", addr);
    let line = format!(r#"{{"type":"mcp","url":"{url}"}}"#);
    {
        let stdout = std::io::stdout();
        let mut h = stdout.lock();
        h.write_all(line.as_bytes())?;
        h.write_all(b"\n")?;
        h.flush()?;
    }

    axum::serve(listener, router).await
}
