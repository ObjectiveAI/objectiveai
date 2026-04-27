mod objectiveai;

use std::sync::Arc;
use envconfig::Envconfig;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
    session::local::LocalSessionManager,
};
use tokio_util::sync::CancellationToken;
use crate::objectiveai::ObjectiveAiMcpCli;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
}

impl EnvConfigBuilder {
    fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            address: self.address,
            port: self.port,
        }
    }
}

#[derive(Default)]
struct ConfigBuilder {
    address: Option<String>,
    port: Option<u16>,
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
    fn build(self) -> Config {
        Config {
            address: self.address.unwrap_or_else(|| "0.0.0.0".into()),
            port: self.port.unwrap_or(3000),
        }
    }
}

struct Config {
    address: String,
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let _ = dotenv::dotenv();
    let config = ConfigBuilder::init_from_env()
        .unwrap_or_default()
        .build();
    let cli_config = Arc::new(
        objectiveai_cli::ConfigBuilder::init_from_env()
            .unwrap_or_default()
            .build(),
    );

    tracing::info!(
        "Starting ObjectiveAI CLI MCP server on {}:{}",
        config.address,
        config.port,
    );

    let server = ObjectiveAiMcpCli::new(cli_config);
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

    let router = axum::Router::new().nest_service("/", service);
    let listener = tokio::net::TcpListener::bind(format!(
        "{}:{}",
        config.address, config.port,
    ))
    .await?;
    tracing::info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;

    Ok(())
}
