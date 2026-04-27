mod mcp;
mod session;
mod session_manager;
mod upstream;

use std::sync::Arc;
use std::time::Duration;

use envconfig::Envconfig;
use objectiveai::mcp::Client;
use tokio_util::sync::CancellationToken;

use crate::session_manager::SessionManager;

/// Shared state every axum handler reaches via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<SessionManager>,
    pub client: Arc<Client>,
}

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

/// Build the shared upstream MCP client. Defaults match
/// `Connection::new_for_test`'s backoff (500 ms initial, 1.5x multiplier,
/// 60 s max interval, 900 s elapsed budget) which has held up well in
/// the existing client-side use cases.
fn build_client() -> Client {
    Client::new(
        reqwest::Client::new(),
        format!("objectiveai-mcp-proxy/{}", env!("CARGO_PKG_VERSION")),
        "ObjectiveAI MCP Proxy".into(),
        "https://objectiveai.dev".into(),
        Duration::from_secs(30),       // connect_timeout
        Duration::from_millis(500),    // backoff_current_interval
        Duration::from_millis(500),    // backoff_initial_interval
        0.5,                           // backoff_randomization_factor
        1.5,                           // backoff_multiplier
        Duration::from_secs(60),       // backoff_max_interval
        Duration::from_secs(900),      // backoff_max_elapsed_time
        Duration::from_secs(30),       // call_timeout
    )
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

    tracing::info!(
        "Starting ObjectiveAI MCP proxy on {}:{}",
        config.address,
        config.port,
    );

    let state = AppState {
        sessions: Arc::new(SessionManager::new()),
        client: Arc::new(build_client()),
    };

    let ct = CancellationToken::new();
    let router = axum::Router::new()
        .route(
            "/",
            axum::routing::post(mcp::handle_post)
                .get(mcp::handle_get)
                .delete(mcp::handle_delete),
        )
        .with_state(state);
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
