//! ObjectiveAI MCP proxy server.
//!
//! Multiplexes a downstream MCP client across one or more upstream MCP
//! servers selected per-request via `X-MCP-Servers` /
//! `X-MCP-Authorization` / `X-MCP-Headers`.
//!
//! Mirrors the `objectiveai-api` `run.rs` shape so other crates can
//! `use objectiveai_mcp_proxy::{ConfigBuilder, run}` and spawn the
//! server in-process without going through the binary.

use std::sync::Arc;
use std::time::Duration;

use envconfig::Envconfig;
use objectiveai::mcp::Client;

use crate::session_manager::SessionManager;
use crate::{AppState, mcp};

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "USER_AGENT")]
    user_agent: Option<String>,
    #[envconfig(from = "HTTP_REFERER")]
    http_referer: Option<String>,
    #[envconfig(from = "X_TITLE")]
    x_title: Option<String>,
    #[envconfig(from = "MCP_CONNECT_TIMEOUT")]
    mcp_connect_timeout: Option<u64>,
    #[envconfig(from = "MCP_CALL_TIMEOUT")]
    mcp_call_timeout: Option<u64>,
    #[envconfig(from = "MCP_BACKOFF_CURRENT_INTERVAL")]
    mcp_backoff_current_interval: Option<u64>,
    #[envconfig(from = "MCP_BACKOFF_INITIAL_INTERVAL")]
    mcp_backoff_initial_interval: Option<u64>,
    #[envconfig(from = "MCP_BACKOFF_RANDOMIZATION_FACTOR")]
    mcp_backoff_randomization_factor: Option<f64>,
    #[envconfig(from = "MCP_BACKOFF_MULTIPLIER")]
    mcp_backoff_multiplier: Option<f64>,
    #[envconfig(from = "MCP_BACKOFF_MAX_INTERVAL")]
    mcp_backoff_max_interval: Option<u64>,
    #[envconfig(from = "MCP_BACKOFF_MAX_ELAPSED_TIME")]
    mcp_backoff_max_elapsed_time: Option<u64>,
    #[envconfig(from = "SUPPRESS_OUTPUT")]
    suppress_output: Option<String>,
}

impl EnvConfigBuilder {
    fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            address: self.address,
            port: self.port,
            user_agent: self.user_agent,
            http_referer: self.http_referer,
            x_title: self.x_title,
            mcp_connect_timeout: self.mcp_connect_timeout,
            mcp_call_timeout: self.mcp_call_timeout,
            mcp_backoff_current_interval: self.mcp_backoff_current_interval,
            mcp_backoff_initial_interval: self.mcp_backoff_initial_interval,
            mcp_backoff_randomization_factor: self.mcp_backoff_randomization_factor,
            mcp_backoff_multiplier: self.mcp_backoff_multiplier,
            mcp_backoff_max_interval: self.mcp_backoff_max_interval,
            mcp_backoff_max_elapsed_time: self.mcp_backoff_max_elapsed_time,
            suppress_output: self.suppress_output.map(|v| {
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
            }),
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub user_agent: Option<String>,
    pub http_referer: Option<String>,
    pub x_title: Option<String>,
    pub mcp_connect_timeout: Option<u64>,
    pub mcp_call_timeout: Option<u64>,
    pub mcp_backoff_current_interval: Option<u64>,
    pub mcp_backoff_initial_interval: Option<u64>,
    pub mcp_backoff_randomization_factor: Option<f64>,
    pub mcp_backoff_multiplier: Option<f64>,
    pub mcp_backoff_max_interval: Option<u64>,
    pub mcp_backoff_max_elapsed_time: Option<u64>,
    pub suppress_output: Option<bool>,
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
            user_agent: self
                .user_agent
                .unwrap_or_else(|| format!("objectiveai-mcp-proxy/{}", env!("CARGO_PKG_VERSION"))),
            http_referer: self
                .http_referer
                .unwrap_or_else(|| "https://objectiveai.dev".to_string()),
            x_title: self
                .x_title
                .unwrap_or_else(|| "ObjectiveAI MCP Proxy".to_string()),
            mcp_connect_timeout: self.mcp_connect_timeout.unwrap_or(30000),
            mcp_call_timeout: self.mcp_call_timeout.unwrap_or(30000),
            mcp_backoff_current_interval: self.mcp_backoff_current_interval.unwrap_or(500),
            mcp_backoff_initial_interval: self.mcp_backoff_initial_interval.unwrap_or(500),
            mcp_backoff_randomization_factor: self.mcp_backoff_randomization_factor.unwrap_or(0.5),
            mcp_backoff_multiplier: self.mcp_backoff_multiplier.unwrap_or(1.5),
            mcp_backoff_max_interval: self.mcp_backoff_max_interval.unwrap_or(60_000),
            mcp_backoff_max_elapsed_time: self.mcp_backoff_max_elapsed_time.unwrap_or(900_000),
            suppress_output: self.suppress_output.unwrap_or(false),
        }
    }
}

pub struct Config {
    pub address: String,
    pub port: u16,
    pub user_agent: String,
    pub http_referer: String,
    pub x_title: String,
    pub mcp_connect_timeout: u64,
    pub mcp_call_timeout: u64,
    pub mcp_backoff_current_interval: u64,
    pub mcp_backoff_initial_interval: u64,
    pub mcp_backoff_randomization_factor: f64,
    pub mcp_backoff_multiplier: f64,
    pub mcp_backoff_max_interval: u64,
    pub mcp_backoff_max_elapsed_time: u64,
    pub suppress_output: bool,
}

pub async fn setup(config: Config) -> std::io::Result<(tokio::net::TcpListener, axum::Router)> {
    let Config {
        address,
        port,
        user_agent,
        http_referer,
        x_title,
        mcp_connect_timeout,
        mcp_call_timeout,
        mcp_backoff_current_interval,
        mcp_backoff_initial_interval,
        mcp_backoff_randomization_factor,
        mcp_backoff_multiplier,
        mcp_backoff_max_interval,
        mcp_backoff_max_elapsed_time,
        suppress_output: _,
    } = config;

    let client = Client::new(
        reqwest::Client::new(),
        user_agent,
        x_title,
        http_referer,
        Duration::from_millis(mcp_connect_timeout),
        Duration::from_millis(mcp_backoff_current_interval),
        Duration::from_millis(mcp_backoff_initial_interval),
        mcp_backoff_randomization_factor,
        mcp_backoff_multiplier,
        Duration::from_millis(mcp_backoff_max_interval),
        Duration::from_millis(mcp_backoff_max_elapsed_time),
        Duration::from_millis(mcp_call_timeout),
    );

    let state = AppState {
        sessions: Arc::new(SessionManager::new()),
        client: Arc::new(client),
    };

    let router = axum::Router::new()
        .route(
            "/",
            axum::routing::post(mcp::handle_post)
                .get(mcp::handle_get)
                .delete(mcp::handle_delete),
        )
        .route("/notify", axum::routing::post(mcp::handle_notify))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;

    Ok((listener, router))
}

pub async fn serve(listener: tokio::net::TcpListener, app: axum::Router) -> std::io::Result<()> {
    axum::serve(listener, app).await
}

pub async fn run(config: Config) -> std::io::Result<()> {
    let suppress_output = config.suppress_output;
    let (listener, app) = setup(config).await?;
    if !suppress_output {
        let addr = listener.local_addr()?;
        eprintln!("listening on {addr}");
    }
    serve(listener, app).await
}
