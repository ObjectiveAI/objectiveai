//! ObjectiveAI MCP proxy server.
//!
//! Multiplexes a downstream MCP client across one or more upstream MCP
//! servers selected per-request via `X-MCP-Servers` /
//! `X-MCP-Headers`.
//!
//! Mirrors the `objectiveai-api` `run.rs` shape so other crates can
//! `use objectiveai_mcp_proxy::{ConfigBuilder, run}` and spawn the
//! server in-process without going through the binary.

use std::sync::Arc;
use std::time::Duration;

use envconfig::Envconfig;
use objectiveai_sdk::mcp::Client;

use crate::session_manager::SessionManager;
use crate::{AppState, mcp};

// Backoff knobs other than the max-elapsed-time are no longer configurable;
// the MCP retry policy uses these fixed defaults. Only
// `MCP_BACKOFF_MAX_ELAPSED_TIME` stays env-configurable. Kept in lockstep
// with objectiveai-api/src/run.rs.
const BACKOFF_INITIAL_INTERVAL_MS: u64 = 100;
const BACKOFF_RANDOMIZATION_FACTOR: f64 = 0.5;
const BACKOFF_MULTIPLIER: f64 = 1.5;
const BACKOFF_MAX_INTERVAL_MS: u64 = 1000;
const BACKOFF_MAX_ELAPSED_TIME_DEFAULT_MS: u64 = 60000;

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
    #[envconfig(from = "MCP_BACKOFF_MAX_ELAPSED_TIME")]
    mcp_backoff_max_elapsed_time: Option<u64>,
    /// Base64-encoded 32-byte key. Used to AEAD-encrypt the proxy
    /// session id payload (per-upstream `Mcp-Session-Id` +
    /// `Authorization` + custom headers).
    ///
    /// Rotation: set a new key, restart the proxy. All outstanding
    /// session ids minted under the old key become 401s; clients
    /// re-initialize.
    ///
    /// Unset → the proxy generates one ephemeral 32-byte key on
    /// startup. Sessions minted by such a process can't be decoded by
    /// any other process or after a restart — which is fine for tests
    /// and dev but bad for production.
    #[envconfig(from = "MCP_ENCRYPTION_KEY")]
    mcp_encryption_key: Option<String>,
    #[envconfig(from = "SUPPRESS_OUTPUT")]
    suppress_output: Option<String>,
    /// When set, the proxy appends a JSONL request/response trace to
    /// `<dir>/mcp-proxy.jsonl`. Unset → logging off. (The embedding API
    /// passes this through [`ConfigBuilder::logs_dir`] instead.)
    #[envconfig(from = "OBJECTIVEAI_LOGS_DIR")]
    logs_dir: Option<String>,
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
            mcp_backoff_max_elapsed_time: self.mcp_backoff_max_elapsed_time,
            mcp_encryption_key: match self.mcp_encryption_key.as_deref() {
                Some(s) => match crate::session_manager::parse_key_env(s) {
                    Ok(opt) => opt,
                    Err(e) => {
                        tracing::error!(error = %e, "MCP_ENCRYPTION_KEY parse failed; falling back to ephemeral key");
                        None
                    }
                },
                None => None,
            },
            suppress_output: self.suppress_output.map(|v| {
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
            }),
            logs_dir: self.logs_dir,
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
    pub mcp_backoff_max_elapsed_time: Option<u64>,
    /// 256-bit AEAD key. `None` → the proxy generates one ephemeral
    /// key per process. See [`EnvConfigBuilder`]'s `mcp_encryption_key`
    /// doc.
    pub mcp_encryption_key: Option<[u8; 32]>,
    pub suppress_output: Option<bool>,
    /// Directory for the proxy's JSONL request/response trace. `Some`
    /// → append to `<dir>/mcp-proxy.jsonl`; `None` → logging off.
    pub logs_dir: Option<String>,
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
            // Defaults match `objectiveai-api/src/run.rs` so the same
            // env vars produce the same effective config when read by
            // either binary independently.
            mcp_connect_timeout: self.mcp_connect_timeout.unwrap_or(60000),
            mcp_call_timeout: self.mcp_call_timeout.unwrap_or(60000),
            mcp_backoff_max_elapsed_time: self.mcp_backoff_max_elapsed_time.unwrap_or(BACKOFF_MAX_ELAPSED_TIME_DEFAULT_MS),
            mcp_encryption_key: self.mcp_encryption_key,
            suppress_output: self.suppress_output.unwrap_or(false),
            logs_dir: self.logs_dir.map(std::path::PathBuf::from),
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
    pub mcp_backoff_max_elapsed_time: u64,
    /// `None` → caller / proxy will generate one ephemeral key.
    pub mcp_encryption_key: Option<[u8; 32]>,
    pub suppress_output: bool,
    /// `Some` → append a JSONL request/response trace to
    /// `<dir>/mcp-proxy.jsonl`; `None` → logging off.
    pub logs_dir: Option<std::path::PathBuf>,
}

pub async fn setup(
    config: Config,
    queue_delegate: Option<std::sync::Arc<dyn crate::QueueDelegate>>,
    reverse_channel: Option<crate::ReverseChannel>,
    dropper: Option<crate::Dropper>,
) -> std::io::Result<(tokio::net::TcpListener, axum::Router)> {
    let Config {
        address,
        port,
        user_agent,
        http_referer,
        x_title,
        mcp_connect_timeout,
        mcp_call_timeout,
        mcp_backoff_max_elapsed_time,
        // Vestigial: the proxy no longer encrypts session ids (they're
        // plain UUIDs keyed by objectiveai response id). The field is
        // kept on Config so the API/CLI env plumbing compiles unchanged;
        // it's ignored here. Removal is deferred to a later step of the
        // session-id refactor.
        mcp_encryption_key: _,
        suppress_output: _,
        logs_dir,
    } = config;

    let client = Client::new(
        reqwest::Client::new(),
        user_agent,
        x_title,
        http_referer,
        Duration::from_millis(mcp_connect_timeout),
        Duration::from_millis(BACKOFF_INITIAL_INTERVAL_MS),
        Duration::from_millis(BACKOFF_INITIAL_INTERVAL_MS),
        BACKOFF_RANDOMIZATION_FACTOR,
        BACKOFF_MULTIPLIER,
        Duration::from_millis(BACKOFF_MAX_INTERVAL_MS),
        Duration::from_millis(mcp_backoff_max_elapsed_time),
        Duration::from_millis(mcp_call_timeout),
    );

    let sessions = Arc::new(SessionManager::new());
    // Late-bind the dropper's teardown context now that sessions + the
    // reverse channel exist (the caller already holds the dropper handle).
    if let Some(dropper) = &dropper {
        dropper.wire(sessions.clone(), reverse_channel.clone());
    }
    // Late-bind the session registry into the reverse channel so inbound
    // MCP-op client_requests can resolve a session by response id.
    if let Some(rc) = &reverse_channel {
        rc.wire_sessions(sessions.clone());
    }
    let state = AppState {
        sessions,
        client: Arc::new(client),
        queue_delegate,
        reverse_channel,
        dropper,
    };

    let router = axum::Router::new()
        .route(
            "/",
            axum::routing::post(mcp::handle_post)
                .get(mcp::handle_get)
                .delete(mcp::handle_delete),
        )
        .with_state(state);

    // Optional JSONL request/response trace. Layered after `with_state`
    // so it wraps every route with its own (logger) state, independent
    // of the handlers' `AppState`.
    let router = match logs_dir {
        Some(dir) => router.layer(axum::middleware::from_fn_with_state(
            std::sync::Arc::new(crate::logging::ProxyLogger::new(dir)),
            crate::logging::log_layer,
        )),
        None => router,
    };

    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;

    Ok((listener, router))
}

pub async fn serve(listener: tokio::net::TcpListener, app: axum::Router) -> std::io::Result<()> {
    axum::serve(listener, app).await
}

pub async fn run(config: Config) -> std::io::Result<()> {
    let suppress_output = config.suppress_output;
    // Bin entry — standalone proxy with no queue delegate / dropper.
    let (listener, app) = setup(config, None, None, None).await?;
    if !suppress_output {
        let addr = listener.local_addr()?;
        eprintln!("listening on {addr}");
    }
    serve(listener, app).await
}
