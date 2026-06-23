//! ObjectiveAI MCP filesystem server.
//!
//! Mirrors the `objectiveai-mcp-proxy` `run.rs` shape so other crates can
//! `use objectiveai_mcp_filesystem::{ConfigBuilder, run}` and spawn the
//! server in-process without going through the binary.

use envconfig::Envconfig;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
    session::local::LocalSessionManager,
};
use tokio_util::sync::CancellationToken;

use crate::tools::FilesystemMcp;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "SUPPRESS_OUTPUT")]
    suppress_output: Option<String>,
}

impl EnvConfigBuilder {
    fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            address: self.address,
            port: self.port,
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
            suppress_output: self.suppress_output.unwrap_or(false),
        }
    }
}

pub struct Config {
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
}

pub async fn setup(config: Config) -> std::io::Result<(tokio::net::TcpListener, axum::Router)> {
    let Config {
        address,
        port,
        suppress_output: _,
    } = config;

    let server = FilesystemMcp::new();
    server.init().await;

    let ct = CancellationToken::new();

    let service: StreamableHttpService<FilesystemMcp, LocalSessionManager> =
        StreamableHttpService::new(move || Ok(server.clone()), Default::default(), {
            // rmcp 1.7 marks `StreamableHttpServerConfig` `#[non_exhaustive]`.
            let mut cfg = StreamableHttpServerConfig::default();
            cfg.stateful_mode = true;
            cfg.sse_keep_alive = None;
            cfg.cancellation_token = ct.child_token();
            cfg
        });

    // axum 0.8 removed nest_service at "/"; fallback_service mounts the
    // service at the root catch-all without the path-prefix-stripping
    // semantics nest_service had (which we never needed since the rmcp
    // service handles every path it cares about itself).
    let router = axum::Router::new().fallback_service(service);
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
