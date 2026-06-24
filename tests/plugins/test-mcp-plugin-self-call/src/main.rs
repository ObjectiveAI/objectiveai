//! Test-fixture plugin that re-enters the per-`response_id` MCP socket
//! mechanic. Its tools read `X-OBJECTIVEAI-RESPONSE-ID` from the
//! incoming request headers and, via the objectiveai `PluginExecutor`,
//! re-invoke `agents tools|resources …` for that response id — which
//! routes back through the host's listener socket into the same agent's
//! MCP aggregation.
//!
//! One binary, four surfaces selected by the mcp server name in
//! `<exe> mcp <server_name> begin` (argv[2]):
//!
//! - `call-other`     — two tools: `hello` (prints "hello world") and
//!   `call_hello` (calls `hello` back through the system via
//!   `agents tools call`).
//! - `list-tools`     — one tool: returns `agents tools list` output.
//! - `list-resources` — one tool: returns `agents resources list`
//!   output (the server also declares a resource).
//! - `read-resource`  — one tool: returns `agents resources read`
//!   output (lists first to discover the aggregated URI, then reads it).
//!
//! Like the other fixtures it writes its PID to `OAI_TEST_MCP_PID_FILE`
//! and announces `{"type":"mcp","url":…}` on stdout before serving.

use std::io::Write;

use objectiveai_sdk::cli::command::agents::resources::list as resources_list;
use objectiveai_sdk::cli::command::agents::resources::read as resources_read;
use objectiveai_sdk::cli::command::agents::tools::call as tools_call;
use objectiveai_sdk::cli::command::agents::tools::list as tools_list;
use objectiveai_sdk::cli::command::plugin::PluginExecutor;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::tool::Extension,
    handler::server::wrapper::Parameters,
    model::{
        Implementation, ListResourcesResult, PaginatedRequestParams, ProtocolVersion,
        ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities,
        ServerInfo,
    },
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use tokio_util::sync::CancellationToken;

/// serverInfo.name — also the aggregated tool-name prefix the proxy
/// assigns (`<serverInfo.name>_<tool>`).
const SELF_NAME: &str = "test-mcp-plugin-self-call";
/// Aggregated name of the `hello` tool as seen across the system.
const HELLO_AGGREGATED: &str = "test-mcp-plugin-self-call_hello";
/// The resource this fixture exposes (for the resource surfaces).
const RESOURCE_URI: &str = "str:///hello";
const RESOURCE_NAME: &str = "hello-resource";
const RESOURCE_TEXT: &str = "resource hello world";

/// Empty tool input — these tools take no parameters of their own.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NoArgs {}

/// Read `X-OBJECTIVEAI-RESPONSE-ID` off the inbound request headers.
fn response_id(parts: &http::request::Parts) -> String {
    parts
        .headers
        .get("x-objectiveai-response-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

// --- executor ops: re-enter the system for `response_id` ---------------

async fn op_call_tool(exec: &PluginExecutor, response_id: String, tool_name: &str) -> String {
    let params: objectiveai_sdk::mcp::tool::CallToolRequestParams =
        serde_json::from_value(serde_json::json!({ "name": tool_name, "arguments": {} }))
            .expect("call-tool params");
    let req = tools_call::Request {
        path_type: tools_call::Path::AgentsToolsCall,
        response_id,
        params,
        base: Default::default(),
    };
    match tools_call::execute(exec, req, None).await {
        Ok(result) => {
            serde_json::to_string(&result).unwrap_or_else(|e| format!("serialize error: {e}"))
        }
        Err(e) => format!("executor error: {e}"),
    }
}

async fn op_list_tools(exec: &PluginExecutor, response_id: String) -> String {
    let params: objectiveai_sdk::mcp::tool::ListToolsRequest =
        serde_json::from_value(serde_json::json!({})).expect("list-tools params");
    let req = tools_list::Request {
        path_type: tools_list::Path::AgentsToolsList,
        response_id,
        params,
        base: Default::default(),
    };
    match tools_list::execute(exec, req, None).await {
        Ok(result) => {
            serde_json::to_string(&result).unwrap_or_else(|e| format!("serialize error: {e}"))
        }
        Err(e) => format!("executor error: {e}"),
    }
}

async fn op_list_resources(exec: &PluginExecutor, response_id: String) -> String {
    let params: objectiveai_sdk::mcp::resource::ListResourcesRequest =
        serde_json::from_value(serde_json::json!({})).expect("list-resources params");
    let req = resources_list::Request {
        path_type: resources_list::Path::AgentsResourcesList,
        response_id,
        params,
        base: Default::default(),
    };
    match resources_list::execute(exec, req, None).await {
        Ok(result) => {
            serde_json::to_string(&result).unwrap_or_else(|e| format!("serialize error: {e}"))
        }
        Err(e) => format!("executor error: {e}"),
    }
}

async fn op_read_resource(exec: &PluginExecutor, response_id: String) -> String {
    // Discover the aggregated URI from `resources list` (the proxy may
    // prefix it), then `resources read` it — robust to any prefixing.
    let lparams: objectiveai_sdk::mcp::resource::ListResourcesRequest =
        serde_json::from_value(serde_json::json!({})).expect("list-resources params");
    let lreq = resources_list::Request {
        path_type: resources_list::Path::AgentsResourcesList,
        response_id: response_id.clone(),
        params: lparams,
        base: Default::default(),
    };
    let list = match resources_list::execute(exec, lreq, None).await {
        Ok(r) => r,
        Err(e) => return format!("executor error (list): {e}"),
    };
    let uri = serde_json::to_value(&list)
        .ok()
        .and_then(|v| v["resources"][0]["uri"].as_str().map(str::to_string))
        .unwrap_or_default();

    let rparams: objectiveai_sdk::mcp::resource::ReadResourceRequestParams =
        serde_json::from_value(serde_json::json!({ "uri": uri })).expect("read-resource params");
    let rreq = resources_read::Request {
        path_type: resources_read::Path::AgentsResourcesRead,
        response_id,
        params: rparams,
        base: Default::default(),
    };
    match resources_read::execute(exec, rreq, None).await {
        Ok(result) => {
            serde_json::to_string(&result).unwrap_or_else(|e| format!("serialize error: {e}"))
        }
        Err(e) => format!("executor error (read): {e}"),
    }
}

fn server_info(resources: bool) -> ServerInfo {
    // rmcp 1.7 marks `ServerInfo`/`Implementation` `#[non_exhaustive]`.
    let mut implementation = Implementation::default();
    implementation.name = SELF_NAME.into();
    implementation.version = env!("CARGO_PKG_VERSION").into();
    let caps = if resources {
        ServerCapabilities::builder().enable_tools().enable_resources().build()
    } else {
        ServerCapabilities::builder().enable_tools().build()
    };
    let mut info = ServerInfo::default();
    info.protocol_version = ProtocolVersion::V_2025_06_18;
    info.capabilities = caps;
    info.server_info = implementation;
    info.instructions = None;
    info
}

fn one_resource_list() -> ListResourcesResult {
    serde_json::from_value(serde_json::json!({
        "resources": [{ "uri": RESOURCE_URI, "name": RESOURCE_NAME, "mimeType": "text" }]
    }))
    .expect("list resources result")
}

// --- surface: call-other (two tools) -----------------------------------

#[derive(Clone)]
struct CallOther {
    tool_router: ToolRouter<Self>,
    exec: PluginExecutor,
}

#[tool_router]
impl CallOther {
    fn new(exec: PluginExecutor) -> Self {
        Self { tool_router: Self::tool_router(), exec }
    }

    #[tool(name = "hello", description = "Print hello world.")]
    async fn hello(&self, Parameters(_): Parameters<NoArgs>) -> String {
        "hello world".to_string()
    }

    #[tool(name = "call_hello", description = "Call the hello tool back through the system.")]
    async fn call_hello(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> String {
        op_call_tool(&self.exec, response_id(&parts), HELLO_AGGREGATED).await
    }
}

#[tool_handler]
impl ServerHandler for CallOther {
    fn get_info(&self) -> ServerInfo {
        server_info(false)
    }
}

// --- surface: list-tools -----------------------------------------------

#[derive(Clone)]
struct ListTools {
    tool_router: ToolRouter<Self>,
    exec: PluginExecutor,
}

#[tool_router]
impl ListTools {
    fn new(exec: PluginExecutor) -> Self {
        Self { tool_router: Self::tool_router(), exec }
    }

    #[tool(name = "do_list_tools", description = "Return agents tools list output.")]
    async fn do_list_tools(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> String {
        op_list_tools(&self.exec, response_id(&parts)).await
    }
}

#[tool_handler]
impl ServerHandler for ListTools {
    fn get_info(&self) -> ServerInfo {
        server_info(false)
    }
}

// --- surface: list-resources -------------------------------------------

#[derive(Clone)]
struct ListResources {
    tool_router: ToolRouter<Self>,
    exec: PluginExecutor,
}

#[tool_router]
impl ListResources {
    fn new(exec: PluginExecutor) -> Self {
        Self { tool_router: Self::tool_router(), exec }
    }

    #[tool(name = "do_list_resources", description = "Return agents resources list output.")]
    async fn do_list_resources(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> String {
        op_list_resources(&self.exec, response_id(&parts)).await
    }
}

#[tool_handler]
impl ServerHandler for ListResources {
    fn get_info(&self) -> ServerInfo {
        server_info(true)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        Ok(one_resource_list())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            RESOURCE_TEXT,
            request.uri,
        )]))
    }
}

// --- surface: read-resource --------------------------------------------

#[derive(Clone)]
struct ReadResource {
    tool_router: ToolRouter<Self>,
    exec: PluginExecutor,
}

#[tool_router]
impl ReadResource {
    fn new(exec: PluginExecutor) -> Self {
        Self { tool_router: Self::tool_router(), exec }
    }

    #[tool(name = "do_read_resource", description = "Return agents resources read output.")]
    async fn do_read_resource(
        &self,
        Parameters(_): Parameters<NoArgs>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> String {
        op_read_resource(&self.exec, response_id(&parts)).await
    }
}

#[tool_handler]
impl ServerHandler for ReadResource {
    fn get_info(&self) -> ServerInfo {
        server_info(true)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        Ok(one_resource_list())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            RESOURCE_TEXT,
            request.uri,
        )]))
    }
}

// --- launch ------------------------------------------------------------

fn http_config() -> StreamableHttpServerConfig {
    // rmcp 1.7 marks `StreamableHttpServerConfig` `#[non_exhaustive]`.
    let mut cfg = StreamableHttpServerConfig::default();
    cfg.stateful_mode = true;
    cfg.sse_keep_alive = None;
    cfg.cancellation_token = CancellationToken::new().child_token();
    cfg
}

async fn run_server<H>(
    listener: tokio::net::TcpListener,
    make: impl Fn() -> H + Send + Sync + 'static,
) -> std::io::Result<()>
where
    H: ServerHandler + Clone + Send + Sync + 'static,
{
    let service: StreamableHttpService<H, LocalSessionManager> =
        StreamableHttpService::new(move || Ok(make()), Default::default(), http_config());
    let router = axum::Router::new().fallback_service(service);
    axum::serve(listener, router).await
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let ok = args.len() >= 4 && args[1] == "mcp" && args[3] == "begin";
    if !ok {
        eprintln!("usage: test-mcp-plugin-self-call mcp <server_name> begin");
        std::process::exit(2);
    }
    let name = args[2].clone();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // PID handoff for the test's Drop-guard cleanup, before announcing.
    if let Ok(path) = std::env::var("OAI_TEST_MCP_PID_FILE") {
        let _ = std::fs::write(&path, std::process::id().to_string());
    }

    // Announce the URL on stdout BEFORE creating the PluginExecutor —
    // the executor then owns stdout for the nested-command protocol.
    let line = format!(r#"{{"type":"mcp","url":"http://{addr}"}}"#);
    {
        let stdout = std::io::stdout();
        let mut h = stdout.lock();
        h.write_all(line.as_bytes())?;
        h.write_all(b"\n")?;
        h.flush()?;
    }

    let exec = PluginExecutor::new();

    match name.as_str() {
        "call-other" => {
            let e = exec.clone();
            run_server(listener, move || CallOther::new(e.clone())).await
        }
        "list-tools" => {
            let e = exec.clone();
            run_server(listener, move || ListTools::new(e.clone())).await
        }
        "list-resources" => {
            let e = exec.clone();
            run_server(listener, move || ListResources::new(e.clone())).await
        }
        "read-resource" => {
            let e = exec.clone();
            run_server(listener, move || ReadResource::new(e.clone())).await
        }
        other => {
            eprintln!("unknown mcp server name: {other}");
            std::process::exit(2);
        }
    }
}
