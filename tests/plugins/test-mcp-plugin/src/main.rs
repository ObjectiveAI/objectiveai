//! Test-fixture plugin that exposes a 10-tool RMCP server.
//!
//! Invoked by the CLI's `dial_plugin_upstream` as
//! `<exe> mcp <server_name> begin`. Spins up `rmcp`'s
//! `StreamableHttpService` on `127.0.0.1:0`, announces the bound URL
//! on stdout as a `cli::plugins::Output::Mcp { url }` line
//! (`{"type":"mcp","url":"http://127.0.0.1:<port>"}`), and serves
//! forever.
//!
//! For test cleanup the binary writes its PID to the path given by
//! `OAI_TEST_MCP_PID_FILE` before announcing the URL — the test's
//! `Drop` guard reads it and force-kills the process.

use std::io::Write;

use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EchoIn {
    input: String,
}

#[derive(Debug, Clone)]
struct TestMcp {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl TestMcp {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(name = "tool0", description = "Echo tool 0")]
    async fn tool0(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool0:{}", req.input)
    }
    #[tool(name = "tool1", description = "Echo tool 1")]
    async fn tool1(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool1:{}", req.input)
    }
    #[tool(name = "tool2", description = "Echo tool 2")]
    async fn tool2(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool2:{}", req.input)
    }
    #[tool(name = "tool3", description = "Echo tool 3")]
    async fn tool3(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool3:{}", req.input)
    }
    #[tool(name = "tool4", description = "Echo tool 4")]
    async fn tool4(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool4:{}", req.input)
    }
    #[tool(name = "tool5", description = "Echo tool 5")]
    async fn tool5(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool5:{}", req.input)
    }
    #[tool(name = "tool6", description = "Echo tool 6")]
    async fn tool6(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool6:{}", req.input)
    }
    #[tool(name = "tool7", description = "Echo tool 7")]
    async fn tool7(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool7:{}", req.input)
    }
    #[tool(name = "tool8", description = "Echo tool 8")]
    async fn tool8(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool8:{}", req.input)
    }
    #[tool(name = "tool9", description = "Echo tool 9")]
    async fn tool9(&self, Parameters(req): Parameters<EchoIn>) -> String {
        format!("tool9:{}", req.input)
    }
}

#[tool_handler]
impl ServerHandler for TestMcp {
    fn get_info(&self) -> ServerInfo {
        // rmcp 1.7 marks `ServerInfo`/`Implementation` `#[non_exhaustive]`.
        let mut server_info = Implementation::default();
        server_info.name = "test-mcp-plugin".into();
        server_info.version = env!("CARGO_PKG_VERSION").into();
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::V_2025_06_18;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info.instructions = None;
        info
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let ok = args.len() >= 4 && args[1] == "mcp" && args[3] == "begin";
    if !ok {
        eprintln!("usage: test-mcp-plugin mcp <server_name> begin");
        std::process::exit(2);
    }

    let service: StreamableHttpService<TestMcp, LocalSessionManager> =
        StreamableHttpService::new(move || Ok(TestMcp::new()), Default::default(), {
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

    // PID handoff for the test's Drop-guard cleanup. Write before
    // announcing the URL so the test can race-free read it the moment
    // the CLI sees the `mcp{url}` line.
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
