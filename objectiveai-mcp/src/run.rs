//! ObjectiveAI MCP server.
//!
//! Mirrors the `objectiveai-mcp-proxy` `run.rs` shape so other crates can
//! `use objectiveai_mcp::{ConfigBuilder, run}` and spawn the server
//! in-process without going through the binary.

use std::sync::Arc;

use envconfig::Envconfig;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio_util::sync::CancellationToken;

use crate::objectiveai::ObjectiveAiMcpCli;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "SUPPRESS_OUTPUT")]
    suppress_output: Option<String>,
    #[envconfig(from = "CONFIG_BASE_DIR")]
    config_base_dir: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_NAME")]
    commit_author_name: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_EMAIL")]
    commit_author_email: Option<String>,
}

impl EnvConfigBuilder {
    fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            address: self.address,
            port: self.port,
            suppress_output: self
                .suppress_output
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")),
            config_base_dir: self.config_base_dir,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub suppress_output: Option<bool>,
    pub config_base_dir: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
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
            config_base_dir: self.config_base_dir,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
        }
    }
}

pub struct Config {
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
    pub config_base_dir: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
}

pub async fn setup(config: Config) -> std::io::Result<(tokio::net::TcpListener, axum::Router)> {
    let Config {
        address,
        port,
        suppress_output: _,
        config_base_dir,
        commit_author_name,
        commit_author_email,
    } = config;

    let cli_config = Arc::new(objectiveai_cli::Config {
        config_set_forbidden: false,
        config_base_dir: config_base_dir.clone(),
        commit_author_name: commit_author_name.clone(),
        commit_author_email: commit_author_email.clone(),
        github_authorization: None,
        // Server-wide default. Per-request, `run_cli_and_collect`
        // overrides this with the `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`
        // header when present; otherwise the call stays stamped as
        // `"mcp"`.
        agent_instance_hierarchy: "mcp".to_string(),
        // Same per-request override pattern for the agent_id — populated
        // from `X-OBJECTIVEAI-AGENT-ID` when the upstream stamps it;
        // otherwise stays `None`.
        agent_id: None,
        mcp_session_id: None,
        mcp: true,
    });

    let fs_client = objectiveai_cli::filesystem::Client::new(
        config_base_dir,
        commit_author_name,
        commit_author_email,
    );
    let (plugins, tools) = tokio::join!(
        fs_client.list_plugins(0, usize::MAX),
        fs_client.list_tools(0, usize::MAX),
    );

    let server = ObjectiveAiMcpCli::with_plugins_and_tools(cli_config, plugins, tools);
    let ct = CancellationToken::new();

    let service: StreamableHttpService<ObjectiveAiMcpCli, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(server.clone()),
            Default::default(),
            StreamableHttpServerConfig {
                stateful_mode: true,
                sse_keep_alive: None,
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );

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
