//! ObjectiveAI MCP server.
//!
//! Other crates can `use objectiveai_mcp::{ConfigBuilder, run}` and
//! spawn the server in-process without going through the binary. The
//! executor is a required argument — `main.rs` builds a
//! `BinaryExecutor` from `Config.objectiveai_dir`; other callers can
//! pass any `CommandExecutor` impl.

use std::sync::Arc;

use envconfig::Envconfig;
use objectiveai_sdk::cli::command::CommandExecutor;
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
            address: self.address.unwrap_or_else(|| "127.0.0.1".to_string()),
            port: self.port.unwrap_or(0),
            suppress_output: self.suppress_output.unwrap_or(false),
            // Layout root (OBJECTIVEAI_DIR). Same default as the api
            // and the viewer.
            objectiveai_dir: match self.objectiveai_dir {
                Some(dir) => std::path::PathBuf::from(dir),
                None => dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".objectiveai"),
            },
            objectiveai_state: self
                .objectiveai_state
                .unwrap_or_else(|| "default".to_string()),
        }
    }
}

pub struct Config {
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
    /// Layout root (`OBJECTIVEAI_DIR`); default `~/.objectiveai`.
    pub objectiveai_dir: std::path::PathBuf,
    /// State name (`OBJECTIVEAI_STATE`); default `"default"`.
    pub objectiveai_state: String,
}

/// Build the rmcp `(TcpListener, axum::Router)` pair.
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

    // Shared per-rmcp-session bag of SessionState (the
    // AgentArguments identity bag plus the X-OBJECTIVEAI-MCP-ROOT
    // gate). Populated by the HeaderSessionManager on every
    // initialize (fresh + lazy reconnect); consumed by the tool
    // dispatcher and the hand-written `list_tools` handler.
    let registry = Arc::new(AgentArgumentsRegistry::new());

    let server = ObjectiveAiMcpCli::new(executor.clone(), registry.clone());
    let session_manager = Arc::new(HeaderSessionManager::new(
        registry.clone(),
        server.clone(),
    ));
    let ct = CancellationToken::new();

    let service: StreamableHttpService<ObjectiveAiMcpCli<E>, HeaderSessionManager<E>> =
        StreamableHttpService::new(move || Ok(server.clone()), session_manager, {
            // rmcp 1.7 marks `StreamableHttpServerConfig` `#[non_exhaustive]`.
            let mut cfg = StreamableHttpServerConfig::default();
            cfg.stateful_mode = true;
            cfg.sse_keep_alive = None;
            cfg.cancellation_token = ct.child_token();
            cfg
        });

    let router = axum::Router::new().fallback_service(service);
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;
    Ok((listener, router))
}

pub async fn serve(
    listener: tokio::net::TcpListener,
    app: axum::Router,
) -> std::io::Result<()> {
    // TCP keepalive on every accepted connection (MCP Streamable HTTP
    // + GET SSE): a silently-dead peer surfaces as a socket error
    // instead of an eternally-idle stream.
    use axum::serve::ListenerExt;
    let listener =
        listener.tap_io(|io| objectiveai_sdk::net::set_tcp_keepalive(io));
    axum::serve(listener, app).await
}

pub async fn run<E>(config: Config, executor: E) -> std::io::Result<()>
where
    E: CommandExecutor + Send + Sync + 'static,
    E::Error: std::fmt::Display + Send + 'static,
{
    let suppress_output = config.suppress_output;
    let (listener, app) = setup(config, executor).await?;

    // The daemon spawns exactly one mcp per state and holds it as a
    // leashed child; the URL clients connect with (wildcard binds map
    // to loopback) is announced via the stdout readiness handshake
    // below.
    let addr = listener.local_addr()?;
    let connect_ip = match addr.ip() {
        std::net::IpAddr::V4(v4) if v4.is_unspecified() => {
            std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
        }
        std::net::IpAddr::V6(v6) if v6.is_unspecified() => {
            std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
        }
        ip => ip,
    };
    let connect_url =
        format!("http://{}", std::net::SocketAddr::new(connect_ip, addr.port()));
    // Readiness handshake: the daemon (this server's sole spawner and
    // leash-holder) blocks on this stdout line and caches the address.
    // No lockfile — the daemon owns this process's lifetime outright.
    objectiveai_sdk::process::print_ready(Some(&connect_url));

    if !suppress_output {
        eprintln!("listening on {addr}");
    }
    serve(listener, app).await
}

