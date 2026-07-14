//! ObjectiveAI MCP filesystem server.
//!
//! Mirrors the `objectiveai-mcp-proxy` `run.rs` shape so other crates can
//! `use objectiveai_mcp_laboratory::{ConfigBuilder, run}` and spawn the
//! server in-process without going through the binary.

use envconfig::Envconfig;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
    session::local::LocalSessionManager,
};
use tokio_util::sync::CancellationToken;

use crate::tools::ObjectiveAiMcpLaboratory;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "SUPPRESS_OUTPUT")]
    suppress_output: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_LABORATORY_ID")]
    laboratory_id: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_LABORATORY_CWD")]
    laboratory_cwd: Option<String>,
}

impl EnvConfigBuilder {
    fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            address: self.address,
            port: self.port,
            suppress_output: self.suppress_output.map(|v| {
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
            }),
            laboratory_id: self.laboratory_id,
            laboratory_cwd: self.laboratory_cwd,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub suppress_output: Option<bool>,
    pub laboratory_id: Option<String>,
    pub laboratory_cwd: Option<String>,
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
            laboratory_id: self.laboratory_id,
            laboratory_cwd: self.laboratory_cwd,
        }
    }
}

pub struct Config {
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
    /// Laboratory id (env `OBJECTIVEAI_LABORATORY_ID`) — names the MCP server
    /// `oail-<id>`. `None` when run standalone (no laboratory).
    pub laboratory_id: Option<String>,
    /// Default working directory new agents start in (env
    /// `OBJECTIVEAI_LABORATORY_CWD`, baked in at `laboratories create`,
    /// defaulting to `/`). `None` when run standalone → falls back to the
    /// process cwd.
    pub laboratory_cwd: Option<String>,
}

pub async fn setup(config: Config) -> std::io::Result<(tokio::net::TcpListener, axum::Router)> {
    let Config {
        address,
        port,
        suppress_output: _,
        laboratory_id,
        laboratory_cwd,
    } = config;

    // Env wins; standalone runs (no env) keep the process cwd → `/` behavior.
    let default_cwd = laboratory_cwd
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
        });

    let server = ObjectiveAiMcpLaboratory::new(laboratory_id, default_cwd);
    server.init().await;

    // Start filesystem-change attribution (best-effort; dormant until
    // the container has CAP_SYS_ADMIN). `FAN_MARK_FILESYSTEM` on `/`
    // covers the whole container filesystem.
    crate::attribution::init(std::path::Path::new("/"));

    let ct = CancellationToken::new();

    let service: StreamableHttpService<ObjectiveAiMcpLaboratory, LocalSessionManager> =
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
    // service handles every path it cares about itself). The explicit
    // `/export` + `/import` routes (laboratory→laboratory file transfer)
    // take precedence over the fallback; the MCP itself lives at `/`.
    let router = axum::Router::new()
        .route("/export", axum::routing::get(crate::transfer::export))
        .route("/import", axum::routing::post(crate::transfer::import))
        .route("/filetree", axum::routing::get(crate::filetree::filetree))
        .fallback_service(service);
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
