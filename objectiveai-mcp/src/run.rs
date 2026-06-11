//! ObjectiveAI MCP server.
//!
//! Other crates can `use objectiveai_mcp::{ConfigBuilder, run}` and
//! spawn the server in-process without going through the binary. The
//! executor is a required argument — `main.rs` builds a
//! `BinaryExecutor` from `Config.objectiveai_dir`; other callers can
//! pass any `CommandExecutor` impl.

use std::sync::Arc;

use envconfig::Envconfig;
use futures::StreamExt;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::plugins;
use objectiveai_sdk::cli::command::tools;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;

use crate::agent_args_registry::AgentArgumentsRegistry;
use crate::header_session_manager::HeaderSessionManager;
use crate::objectiveai::ObjectiveAiMcpCli;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "SUPPRESS_OUTPUT")]
    suppress_output: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_DIR")]
    objectiveai_dir: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_STATE")]
    objectiveai_state: Option<String>,
}

impl EnvConfigBuilder {
    fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            address: self.address,
            port: self.port,
            suppress_output: self
                .suppress_output
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")),
            objectiveai_dir: self.objectiveai_dir,
            objectiveai_state: self.objectiveai_state,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub suppress_output: Option<bool>,
    pub objectiveai_dir: Option<String>,
    pub objectiveai_state: Option<String>,
}

impl Envconfig for ConfigBuilder {
    #[allow(deprecated)]
    fn init() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init().map(|e| e.build())
    }

    fn init_from_env() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_env().map(|e| e.build())
    }

    fn init_from_hashmap(
        hashmap: &std::collections::HashMap<String, String>,
    ) -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_hashmap(hashmap).map(|e| e.build())
    }
}

impl ConfigBuilder {
    pub fn build(self) -> Config {
        Config {
            address: self.address.unwrap_or_else(|| "0.0.0.0".to_string()),
            port: self.port.unwrap_or(3000),
            suppress_output: self.suppress_output.unwrap_or(false),
            objectiveai_dir: self.objectiveai_dir,
            objectiveai_state: self.objectiveai_state,
        }
    }
}

pub struct Config {
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
    pub objectiveai_dir: Option<String>,
    pub objectiveai_state: Option<String>,
}

/// Build the rmcp `(TcpListener, axum::Router)` pair. The executor is
/// `Arc`-wrapped internally so the dynamic plugin / tool route
/// closures and the static `ObjectiveAI` handler share a single
/// instance.
pub async fn setup<E>(
    config: Config,
    executor: E,
) -> std::io::Result<(tokio::net::TcpListener, axum::Router)>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    let Config {
        address,
        port,
        suppress_output: _,
        objectiveai_dir: _,
        objectiveai_state: _,
    } = config;

    let executor = Arc::new(executor);

    // Discover registered plugins + tools concurrently — both are
    // independent SDK calls that each spawn one cli subprocess.
    // The lists are shared (via Arc) between `with_plugins_and_tools`
    // (which registers one dynamic tool per entry) and
    // `HeaderSessionManager::new` (which validates the optional
    // `X-OBJECTIVEAI-MCP-{TOOLS,PLUGINS}` filter sets against the
    // same installed manifest at connect time).
    let (plugins_list, tools_list) =
        tokio::join!(list_plugins(&*executor), list_tools(&*executor));
    let plugins_list = Arc::new(plugins_list);
    let tools_list = Arc::new(tools_list);

    // Shared per-rmcp-session bag of SessionState (wraps the
    // legacy AgentArguments identity bag alongside the three
    // optional X-OBJECTIVEAI-MCP-* filter values). Populated by
    // the HeaderSessionManager on every initialize (fresh + lazy
    // reconnect); consumed by every tool dispatcher and the
    // hand-written `list_tools` handler.
    let registry = Arc::new(AgentArgumentsRegistry::new());

    let server = ObjectiveAiMcpCli::with_plugins_and_tools(
        executor.clone(),
        (*plugins_list).clone(),
        (*tools_list).clone(),
        registry.clone(),
    );
    let session_manager = Arc::new(HeaderSessionManager::new(
        registry.clone(),
        server.clone(),
        tools_list.clone(),
        plugins_list.clone(),
    ));
    let ct = CancellationToken::new();

    let service: StreamableHttpService<ObjectiveAiMcpCli<E>, HeaderSessionManager<E>> =
        StreamableHttpService::new(
            move || Ok(server.clone()),
            session_manager,
            StreamableHttpServerConfig {
                stateful_mode: true,
                sse_keep_alive: None,
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );

    let router = axum::Router::new().fallback_service(service);
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;
    Ok((listener, router))
}

pub async fn serve(
    listener: tokio::net::TcpListener,
    app: axum::Router,
) -> std::io::Result<()> {
    axum::serve(listener, app).await
}

pub async fn run<E>(config: Config, executor: E) -> std::io::Result<()>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    let suppress_output = config.suppress_output;
    let (listener, app) = setup(config, executor).await?;
    if !suppress_output {
        let addr = listener.local_addr()?;
        eprintln!("listening on {addr}");
    }
    serve(listener, app).await
}

/// Best-effort plugin discovery: drain `plugins list`, drop per-item
/// errors. A discovery failure (cli binary missing, etc.) returns an
/// empty list so the server still starts.
async fn list_plugins<E>(executor: &E) -> Vec<plugins::list::ResponseItem>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    let request = plugins::list::Request {
        path_type: plugins::list::Path::PluginsList,
        offset: None,
        limit: None,
        jq: None,
    };
    match plugins::list::execute(executor, request, None).await {
        Ok(stream) => stream.filter_map(|r| async move { r.ok() }).collect().await,
        Err(_) => Vec::new(),
    }
}

/// Best-effort tool discovery; same shape as `list_plugins`.
async fn list_tools<E>(executor: &E) -> Vec<tools::list::ResponseItem>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    let request = tools::list::Request {
        path_type: tools::list::Path::ToolsList,
        offset: None,
        limit: None,
        jq: None,
    };
    match tools::list::execute(executor, request, None).await {
        Ok(stream) => stream.filter_map(|r| async move { r.ok() }).collect().await,
        Err(_) => Vec::new(),
    }
}
