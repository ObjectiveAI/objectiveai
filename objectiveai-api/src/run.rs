//! ObjectiveAI API server.
//!
//! REST API server for chat completions, vector completions, Functions,
//! Profiles, Swarms, and authentication.

use axum::{
    Json,
    extract::ws::WebSocketUpgrade,
    response::{IntoResponse, Sse, sse::Event},
};
use envconfig::Envconfig;
use objectiveai_sdk::error::ResponseError;
use crate::{
    agent, auth, ctx,
    error::ResponseErrorExt,
    filesystem,
    functions::{self, profiles::computations::Client},
    github, objectiveai_http,
    retrieval, streaming_ws, streaming_ws_handlers,
    util::StreamOnce,
    vector,
};
use std::{convert::Infallible, sync::Arc};
use tokio_stream::StreamExt;

type ListRouter = retrieval::list::Router<
    retrieval::list::objectiveai::ObjectiveAiClient,
    retrieval::list::filesystem::FilesystemClient,
    retrieval::list::mock::MockClient,
    ctx::DefaultContextExt,
>;

type RetrieveRouter = retrieval::retrieve::Router<
    retrieval::retrieve::github::GithubClient,
    retrieval::retrieve::filesystem::FilesystemClient,
    retrieval::retrieve::mock::MockClient,
    ctx::DefaultContextExt,
>;

type UsageRouter = retrieval::usage::Router<
    retrieval::usage::objectiveai::ObjectiveAiClient,
    ctx::DefaultContextExt,
>;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    // -- HttpClient fields (identical order across all 3 structs) --
    #[envconfig(from = "OBJECTIVEAI_ADDRESS")]
    objectiveai_address: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_AUTHORIZATION")]
    objectiveai_authorization: Option<String>,
    #[envconfig(from = "OPENROUTER_ADDRESS")]
    openrouter_address: Option<String>,
    #[envconfig(from = "OPENROUTER_AUTHORIZATION")]
    openrouter_authorization: Option<String>,
    #[envconfig(from = "GITHUB_AUTHORIZATION")]
    github_authorization: Option<String>,
    #[envconfig(from = "MCP_AUTHORIZATION")]
    mcp_authorization: Option<String>,
    #[envconfig(from = "USER_AGENT")]
    user_agent: Option<String>,
    #[envconfig(from = "HTTP_REFERER")]
    http_referer: Option<String>,
    #[envconfig(from = "X_TITLE")]
    x_title: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_NAME")]
    commit_author_name: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_EMAIL")]
    commit_author_email: Option<String>,
    // -- Other fields --
    #[envconfig(from = "CLAUDE_AGENT_SDK_ENABLED")]
    claude_agent_sdk_enabled: Option<String>,
    #[envconfig(from = "CLAUDE_AGENT_SDK_RATE_LIMIT_MAX_RETRIES")]
    claude_agent_sdk_rate_limit_max_retries: Option<u64>,
    #[envconfig(from = "CLAUDE_AGENT_SDK_RATE_LIMIT_MAX_WAIT_SECS")]
    claude_agent_sdk_rate_limit_max_wait_secs: Option<u64>,
    #[envconfig(from = "CLAUDE_AGENT_SDK_QUERY_LIMIT")]
    claude_agent_sdk_query_limit: Option<u64>,
    #[envconfig(from = "CODEX_SDK_ENABLED")]
    codex_sdk_enabled: Option<String>,
    #[envconfig(from = "CODEX_SDK_RATE_LIMIT_MAX_RETRIES")]
    codex_sdk_rate_limit_max_retries: Option<u64>,
    #[envconfig(from = "CODEX_SDK_RATE_LIMIT_MAX_WAIT_SECS")]
    codex_sdk_rate_limit_max_wait_secs: Option<u64>,
    #[envconfig(from = "CODEX_SDK_QUERY_LIMIT")]
    codex_sdk_query_limit: Option<u64>,
    #[envconfig(from = "AGENT_COMPLETIONS_BACKOFF_CURRENT_INTERVAL")]
    agent_completions_backoff_current_interval: Option<u64>,
    #[envconfig(from = "AGENT_COMPLETIONS_BACKOFF_INITIAL_INTERVAL")]
    agent_completions_backoff_initial_interval: Option<u64>,
    #[envconfig(from = "AGENT_COMPLETIONS_BACKOFF_RANDOMIZATION_FACTOR")]
    agent_completions_backoff_randomization_factor: Option<f64>,
    #[envconfig(from = "AGENT_COMPLETIONS_BACKOFF_MULTIPLIER")]
    agent_completions_backoff_multiplier: Option<f64>,
    #[envconfig(from = "AGENT_COMPLETIONS_BACKOFF_MAX_INTERVAL")]
    agent_completions_backoff_max_interval: Option<u64>,
    #[envconfig(from = "AGENT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME")]
    agent_completions_backoff_max_elapsed_time: Option<u64>,
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
    #[envconfig(from = "GITHUB_BACKOFF_CURRENT_INTERVAL")]
    github_backoff_current_interval: Option<u64>,
    #[envconfig(from = "GITHUB_BACKOFF_INITIAL_INTERVAL")]
    github_backoff_initial_interval: Option<u64>,
    #[envconfig(from = "GITHUB_BACKOFF_RANDOMIZATION_FACTOR")]
    github_backoff_randomization_factor: Option<f64>,
    #[envconfig(from = "GITHUB_BACKOFF_MULTIPLIER")]
    github_backoff_multiplier: Option<f64>,
    #[envconfig(from = "GITHUB_BACKOFF_MAX_INTERVAL")]
    github_backoff_max_interval: Option<u64>,
    #[envconfig(from = "GITHUB_BACKOFF_MAX_ELAPSED_TIME")]
    github_backoff_max_elapsed_time: Option<u64>,
    #[envconfig(from = "AGENT_COMPLETIONS_FIRST_CHUNK_TIMEOUT")]
    agent_completions_first_chunk_timeout: Option<u64>,
    #[envconfig(from = "AGENT_COMPLETIONS_OTHER_CHUNK_TIMEOUT")]
    agent_completions_other_chunk_timeout: Option<u64>,
    #[envconfig(from = "MCP_CONNECT_TIMEOUT")]
    mcp_connect_timeout: Option<u64>,
    #[envconfig(from = "MCP_CALL_TIMEOUT")]
    mcp_call_timeout: Option<u64>,
    #[envconfig(from = "REVERSE_CHANNEL_TIMEOUT")]
    reverse_channel_timeout: Option<u64>,
    #[envconfig(from = "MCP_ENCRYPTION_KEY")]
    mcp_encryption_key: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_DIR")]
    objectiveai_dir: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_STATE")]
    objectiveai_state: Option<String>,
    #[envconfig(from = "PERSISTENT_CACHE_TRANSIENT_TTL_MS")]
    persistent_cache_transient_ttl_ms: Option<u64>,
    #[envconfig(from = "MOCK_DELAY_MS")]
    mock_delay_ms: Option<u64>,
    #[envconfig(from = "MOCK_MAX_TOOL_CALLS")]
    mock_max_tool_calls: Option<u32>,
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
}

impl EnvConfigBuilder {
    pub fn build(self) -> ConfigBuilder {
        fn parse_bool(s: &str) -> bool {
            let v = s.trim();
            !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
        }
        ConfigBuilder {
            // -- HttpClient fields --
            objectiveai_address: self.objectiveai_address,
            objectiveai_authorization: self.objectiveai_authorization,
            openrouter_address: self.openrouter_address,
            openrouter_authorization: self.openrouter_authorization,
            github_authorization: self.github_authorization,
            mcp_authorization: self.mcp_authorization,
            user_agent: self.user_agent,
            http_referer: self.http_referer,
            x_title: self.x_title,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            // -- Other fields --
            claude_agent_sdk_enabled: self.claude_agent_sdk_enabled.map(|s| parse_bool(&s)),
            claude_agent_sdk_rate_limit_max_retries: self.claude_agent_sdk_rate_limit_max_retries,
            claude_agent_sdk_rate_limit_max_wait_secs: self.claude_agent_sdk_rate_limit_max_wait_secs,
            claude_agent_sdk_query_limit: self.claude_agent_sdk_query_limit,
            codex_sdk_enabled: self.codex_sdk_enabled.map(|s| parse_bool(&s)),
            codex_sdk_rate_limit_max_retries: self.codex_sdk_rate_limit_max_retries,
            codex_sdk_rate_limit_max_wait_secs: self.codex_sdk_rate_limit_max_wait_secs,
            codex_sdk_query_limit: self.codex_sdk_query_limit,
            agent_completions_backoff_current_interval: self.agent_completions_backoff_current_interval,
            agent_completions_backoff_initial_interval: self.agent_completions_backoff_initial_interval,
            agent_completions_backoff_randomization_factor: self.agent_completions_backoff_randomization_factor,
            agent_completions_backoff_multiplier: self.agent_completions_backoff_multiplier,
            agent_completions_backoff_max_interval: self.agent_completions_backoff_max_interval,
            agent_completions_backoff_max_elapsed_time: self.agent_completions_backoff_max_elapsed_time,
            mcp_backoff_current_interval: self.mcp_backoff_current_interval,
            mcp_backoff_initial_interval: self.mcp_backoff_initial_interval,
            mcp_backoff_randomization_factor: self.mcp_backoff_randomization_factor,
            mcp_backoff_multiplier: self.mcp_backoff_multiplier,
            mcp_backoff_max_interval: self.mcp_backoff_max_interval,
            mcp_backoff_max_elapsed_time: self.mcp_backoff_max_elapsed_time,
            github_backoff_current_interval: self.github_backoff_current_interval,
            github_backoff_initial_interval: self.github_backoff_initial_interval,
            github_backoff_randomization_factor: self.github_backoff_randomization_factor,
            github_backoff_multiplier: self.github_backoff_multiplier,
            github_backoff_max_interval: self.github_backoff_max_interval,
            github_backoff_max_elapsed_time: self.github_backoff_max_elapsed_time,
            agent_completions_first_chunk_timeout: self.agent_completions_first_chunk_timeout,
            agent_completions_other_chunk_timeout: self.agent_completions_other_chunk_timeout,
            mcp_connect_timeout: self.mcp_connect_timeout,
            mcp_call_timeout: self.mcp_call_timeout,
            reverse_channel_timeout: self.reverse_channel_timeout,
            mcp_encryption_key: self.mcp_encryption_key,
            objectiveai_dir: self.objectiveai_dir,
            objectiveai_state: self.objectiveai_state,
            persistent_cache_transient_ttl_ms: self.persistent_cache_transient_ttl_ms,
            mock_delay_ms: self.mock_delay_ms,
            mock_max_tool_calls: self.mock_max_tool_calls,
            address: self.address,
            port: self.port,
            suppress_output: None,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    // -- HttpClient fields (identical order across all 3 structs) --
    pub objectiveai_address: Option<String>,
    pub objectiveai_authorization: Option<String>,
    pub openrouter_address: Option<String>,
    pub openrouter_authorization: Option<String>,
    pub github_authorization: Option<String>,
    pub mcp_authorization: Option<String>,
    pub user_agent: Option<String>,
    pub http_referer: Option<String>,
    pub x_title: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    // -- Other fields --
    pub claude_agent_sdk_enabled: Option<bool>,
    pub claude_agent_sdk_rate_limit_max_retries: Option<u64>,
    pub claude_agent_sdk_rate_limit_max_wait_secs: Option<u64>,
    pub claude_agent_sdk_query_limit: Option<u64>,
    pub codex_sdk_enabled: Option<bool>,
    pub codex_sdk_rate_limit_max_retries: Option<u64>,
    pub codex_sdk_rate_limit_max_wait_secs: Option<u64>,
    pub codex_sdk_query_limit: Option<u64>,
    pub agent_completions_backoff_current_interval: Option<u64>,
    pub agent_completions_backoff_initial_interval: Option<u64>,
    pub agent_completions_backoff_randomization_factor: Option<f64>,
    pub agent_completions_backoff_multiplier: Option<f64>,
    pub agent_completions_backoff_max_interval: Option<u64>,
    pub agent_completions_backoff_max_elapsed_time: Option<u64>,
    pub mcp_backoff_current_interval: Option<u64>,
    pub mcp_backoff_initial_interval: Option<u64>,
    pub mcp_backoff_randomization_factor: Option<f64>,
    pub mcp_backoff_multiplier: Option<f64>,
    pub mcp_backoff_max_interval: Option<u64>,
    pub mcp_backoff_max_elapsed_time: Option<u64>,
    pub github_backoff_current_interval: Option<u64>,
    pub github_backoff_initial_interval: Option<u64>,
    pub github_backoff_randomization_factor: Option<f64>,
    pub github_backoff_multiplier: Option<f64>,
    pub github_backoff_max_interval: Option<u64>,
    pub github_backoff_max_elapsed_time: Option<u64>,
    pub agent_completions_first_chunk_timeout: Option<u64>,
    pub agent_completions_other_chunk_timeout: Option<u64>,
    pub mcp_connect_timeout: Option<u64>,
    pub mcp_call_timeout: Option<u64>,
    pub reverse_channel_timeout: Option<u64>,
    pub mcp_encryption_key: Option<String>,
    pub objectiveai_dir: Option<String>,
    pub objectiveai_state: Option<String>,
    pub persistent_cache_transient_ttl_ms: Option<u64>,
    pub mock_delay_ms: Option<u64>,
    pub mock_max_tool_calls: Option<u32>,
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

    fn init_from_hashmap(hashmap: &std::collections::HashMap<String, String>) -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_hashmap(hashmap).map(|e| e.build())
    }
}

impl ConfigBuilder {
    pub fn build(self) -> Config {
        Config {
            // -- HttpClient fields --
            objectiveai_address: self.objectiveai_address.unwrap_or_else(|| "https://api.objectiveai.dev".to_string()),
            objectiveai_authorization: self.objectiveai_authorization,
            openrouter_address: self.openrouter_address.unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string()),
            openrouter_authorization: self.openrouter_authorization,
            github_authorization: self.github_authorization,
            mcp_authorization: self.mcp_authorization,
            user_agent: self.user_agent.unwrap_or_else(|| "objectiveai-ai<admin@objectiveai-ai.io>".to_string()),
            http_referer: self.http_referer.unwrap_or_else(|| "https://objectiveai-ai.io/".to_string()),
            x_title: self.x_title.unwrap_or_else(|| "ObjectiveAI".to_string()),
            commit_author_name: self.commit_author_name.unwrap_or_else(|| "ObjectiveAI".to_string()),
            commit_author_email: self.commit_author_email.unwrap_or_else(|| "admin@objectiveai.dev".to_string()),
            // -- Other fields --
            claude_agent_sdk_enabled: self.claude_agent_sdk_enabled.unwrap_or(true),
            claude_agent_sdk_rate_limit_max_retries: self.claude_agent_sdk_rate_limit_max_retries.unwrap_or(10),
            claude_agent_sdk_rate_limit_max_wait_secs: self.claude_agent_sdk_rate_limit_max_wait_secs.unwrap_or(180),
            claude_agent_sdk_query_limit: self.claude_agent_sdk_query_limit.unwrap_or(10),
            codex_sdk_enabled: self.codex_sdk_enabled.unwrap_or(true),
            codex_sdk_rate_limit_max_retries: self.codex_sdk_rate_limit_max_retries.unwrap_or(10),
            codex_sdk_rate_limit_max_wait_secs: self.codex_sdk_rate_limit_max_wait_secs.unwrap_or(180),
            codex_sdk_query_limit: self.codex_sdk_query_limit.unwrap_or(10),
            agent_completions_backoff_current_interval: self.agent_completions_backoff_current_interval.unwrap_or(100),
            agent_completions_backoff_initial_interval: self.agent_completions_backoff_initial_interval.unwrap_or(100),
            agent_completions_backoff_randomization_factor: self.agent_completions_backoff_randomization_factor.unwrap_or(0.5),
            agent_completions_backoff_multiplier: self.agent_completions_backoff_multiplier.unwrap_or(1.5),
            agent_completions_backoff_max_interval: self.agent_completions_backoff_max_interval.unwrap_or(1000),
            agent_completions_backoff_max_elapsed_time: self.agent_completions_backoff_max_elapsed_time.unwrap_or(40000),
            mcp_backoff_current_interval: self.mcp_backoff_current_interval.unwrap_or(100),
            mcp_backoff_initial_interval: self.mcp_backoff_initial_interval.unwrap_or(100),
            mcp_backoff_randomization_factor: self.mcp_backoff_randomization_factor.unwrap_or(0.5),
            mcp_backoff_multiplier: self.mcp_backoff_multiplier.unwrap_or(1.5),
            mcp_backoff_max_interval: self.mcp_backoff_max_interval.unwrap_or(1000),
            mcp_backoff_max_elapsed_time: self.mcp_backoff_max_elapsed_time.unwrap_or(40000),
            github_backoff_current_interval: self.github_backoff_current_interval.unwrap_or(100),
            github_backoff_initial_interval: self.github_backoff_initial_interval.unwrap_or(100),
            github_backoff_randomization_factor: self.github_backoff_randomization_factor.unwrap_or(0.5),
            github_backoff_multiplier: self.github_backoff_multiplier.unwrap_or(1.5),
            github_backoff_max_interval: self.github_backoff_max_interval.unwrap_or(1000),
            github_backoff_max_elapsed_time: self.github_backoff_max_elapsed_time.unwrap_or(40000),
            agent_completions_first_chunk_timeout: self.agent_completions_first_chunk_timeout.unwrap_or(60000),
            agent_completions_other_chunk_timeout: self.agent_completions_other_chunk_timeout.unwrap_or(30000),
            mcp_connect_timeout: self.mcp_connect_timeout.unwrap_or(30000),
            mcp_call_timeout: self.mcp_call_timeout.unwrap_or(30000),
            reverse_channel_timeout: self.reverse_channel_timeout.unwrap_or(30000),
            mcp_encryption_key: self.mcp_encryption_key,
            // Layout root (OBJECTIVEAI_DIR). Kept on Config for the
            // paths that live OUTSIDE the state dir — e.g. the
            // instance lock at <dir>/bin/locks/api/.
            objectiveai_dir: match self.objectiveai_dir.as_deref() {
                Some(dir) => std::path::PathBuf::from(dir),
                None => dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".objectiveai"),
            },
            // The api's filesystem client holds per-state data
            // (functions/, profiles/), so resolve straight to the
            // state dir: <dir>/state/<state>.
            config_base_dir: {
                let dir = match self.objectiveai_dir {
                    Some(dir) => std::path::PathBuf::from(dir),
                    None => dirs::home_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join(".objectiveai"),
                };
                let state = self.objectiveai_state.unwrap_or_else(|| "default".to_string());
                dir.join("state").join(state)
            },
            persistent_cache_transient_ttl_ms: self.persistent_cache_transient_ttl_ms.unwrap_or(3_600_000),
            mock_delay_ms: self.mock_delay_ms.unwrap_or(0),
            mock_max_tool_calls: self.mock_max_tool_calls.unwrap_or(1000),
            // Loopback + ephemeral by default: the actual bound port
            // is read back from the listener and published in the api
            // lock file, so a fixed default is unnecessary.
            address: self.address.unwrap_or_else(|| "127.0.0.1".to_string()),
            port: self.port.unwrap_or(0),
            suppress_output: self.suppress_output.unwrap_or(false),
        }
    }
}

pub struct Config {
    // -- HttpClient fields (identical order across all 3 structs) --
    pub objectiveai_address: String,
    pub objectiveai_authorization: Option<String>,
    pub openrouter_address: String,
    pub openrouter_authorization: Option<String>,
    pub github_authorization: Option<String>,
    pub mcp_authorization: Option<String>,
    pub user_agent: String,
    pub http_referer: String,
    pub x_title: String,
    pub commit_author_name: String,
    pub commit_author_email: String,
    // -- Other fields --
    pub claude_agent_sdk_enabled: bool,
    pub claude_agent_sdk_rate_limit_max_retries: u64,
    pub claude_agent_sdk_rate_limit_max_wait_secs: u64,
    pub claude_agent_sdk_query_limit: u64,
    pub codex_sdk_enabled: bool,
    pub codex_sdk_rate_limit_max_retries: u64,
    pub codex_sdk_rate_limit_max_wait_secs: u64,
    pub codex_sdk_query_limit: u64,
    pub agent_completions_backoff_current_interval: u64,
    pub agent_completions_backoff_initial_interval: u64,
    pub agent_completions_backoff_randomization_factor: f64,
    pub agent_completions_backoff_multiplier: f64,
    pub agent_completions_backoff_max_interval: u64,
    pub agent_completions_backoff_max_elapsed_time: u64,
    pub mcp_backoff_current_interval: u64,
    pub mcp_backoff_initial_interval: u64,
    pub mcp_backoff_randomization_factor: f64,
    pub mcp_backoff_multiplier: f64,
    pub mcp_backoff_max_interval: u64,
    pub mcp_backoff_max_elapsed_time: u64,
    pub github_backoff_current_interval: u64,
    pub github_backoff_initial_interval: u64,
    pub github_backoff_randomization_factor: f64,
    pub github_backoff_multiplier: f64,
    pub github_backoff_max_interval: u64,
    pub github_backoff_max_elapsed_time: u64,
    pub agent_completions_first_chunk_timeout: u64,
    pub agent_completions_other_chunk_timeout: u64,
    pub mcp_connect_timeout: u64,
    pub mcp_call_timeout: u64,
    /// Budget (ms) for one WS reverse-channel round-trip — how long
    /// a forwarded MCP server-request or a message-queue read may
    /// wait for the CLI's reply. Long enough that a healthy but
    /// heavily loaded CLI answers in time, short enough that a
    /// wedged WS doesn't stall callers indefinitely.
    pub reverse_channel_timeout: u64,
    /// Base64-encoded 32-byte key. Forwarded to the spawned proxy as
    /// `MCP_ENCRYPTION_KEY`. Unset → proxy generates an ephemeral key
    /// per process.
    pub mcp_encryption_key: Option<String>,
    /// Layout root (`OBJECTIVEAI_DIR`); `config_base_dir` is the
    /// per-state dir derived from it.
    pub objectiveai_dir: std::path::PathBuf,
    pub config_base_dir: std::path::PathBuf,
    pub persistent_cache_transient_ttl_ms: u64,
    pub mock_delay_ms: u64,
    pub mock_max_tool_calls: u32,
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
}

pub async fn setup(
    config: Config,
) -> std::io::Result<(
    tokio::net::TcpListener,
    axum::Router,
    tokio::net::TcpListener,
    axum::Router,
)> {
    let Config {
        // -- HttpClient fields --
        objectiveai_address,
        objectiveai_authorization,
        openrouter_address,
        openrouter_authorization,
        github_authorization,
        mcp_authorization,
        user_agent,
        http_referer,
        x_title,
        commit_author_name,
        commit_author_email,
        // -- Other fields --
        claude_agent_sdk_enabled,
        claude_agent_sdk_rate_limit_max_retries,
        claude_agent_sdk_rate_limit_max_wait_secs,
        claude_agent_sdk_query_limit,
        codex_sdk_enabled,
        codex_sdk_rate_limit_max_retries,
        codex_sdk_rate_limit_max_wait_secs,
        codex_sdk_query_limit,
        agent_completions_backoff_current_interval,
        agent_completions_backoff_initial_interval,
        agent_completions_backoff_randomization_factor,
        agent_completions_backoff_multiplier,
        agent_completions_backoff_max_interval,
        agent_completions_backoff_max_elapsed_time,
        mcp_backoff_current_interval,
        mcp_backoff_initial_interval,
        mcp_backoff_randomization_factor,
        mcp_backoff_multiplier,
        mcp_backoff_max_interval,
        mcp_backoff_max_elapsed_time,
        github_backoff_current_interval,
        github_backoff_initial_interval,
        github_backoff_randomization_factor,
        github_backoff_multiplier,
        github_backoff_max_interval,
        github_backoff_max_elapsed_time,
        agent_completions_first_chunk_timeout,
        agent_completions_other_chunk_timeout,
        mcp_connect_timeout,
        mcp_call_timeout,
        reverse_channel_timeout,
        mcp_encryption_key,
        objectiveai_dir: _,
        config_base_dir,
        persistent_cache_transient_ttl_ms,
        mock_delay_ms,
        mock_max_tool_calls,
        address,
        port,
        suppress_output,
    } = config;

    // Publish the WS reverse-channel budget for its two crate-wide
    // consumers (the MCP forward path and the agent client's
    // message-queue reads).
    crate::objectiveai_mcp::set_reverse_channel_timeout(
        std::time::Duration::from_millis(reverse_channel_timeout),
    );

    // HTTP Client
    let http_client = reqwest::Client::new();

    // Parse MCP authorization (shared between objectiveai_http and agent_completions clients)
    let mcp_authorization: Option<Arc<std::collections::HashMap<String, String>>> = mcp_authorization
        .and_then(|s| serde_json::from_str(&s).ok())
        .map(Arc::new);

    // ObjectiveAI HTTP Client
    let objectiveai_http_client = Arc::new(objectiveai_http::Client::new(
        http_client.clone(),
        objectiveai_address,
        objectiveai_authorization,
        user_agent.clone(),
        x_title.clone(),
        http_referer.clone(),
        github_authorization.as_ref().map(|s| Arc::new(s.clone())),
        openrouter_authorization.as_ref().map(|s| Arc::new(s.clone())),
        mcp_authorization.clone(),
        Some(Arc::new(commit_author_name.clone())),
        Some(Arc::new(commit_author_email.clone())),
    ));

    // Vector Completion Votes Fetcher
    let completion_votes_fetcher = Arc::new(
        vector::completions::completion_votes_fetcher::ObjectiveAiFetcher::new(
            objectiveai_http_client.clone(),
        ),
    );

    // Vector Cache Vote Fetcher
    let cache_vote_fetcher = Arc::new(
        vector::completions::cache_vote_fetcher::ObjectiveAiFetcher::new(
            objectiveai_http_client.clone(),
        ),
    );

    // GitHub Client
    let github_client = Arc::new(github::Client::new(
        http_client.clone(),
        github_authorization.clone(),
        true, // allow_publish_without_byok
        user_agent.clone(),
        x_title.clone(),
        http_referer.clone(),
        std::time::Duration::from_millis(github_backoff_current_interval),
        std::time::Duration::from_millis(github_backoff_initial_interval),
        github_backoff_randomization_factor,
        github_backoff_multiplier,
        std::time::Duration::from_millis(github_backoff_max_interval),
        std::time::Duration::from_millis(github_backoff_max_elapsed_time),
    ));

    let filesystem_client = Arc::new(filesystem::Client::new(
        config_base_dir.clone(),
        commit_author_name,
        commit_author_email,
    ));


    // Retrieval: Retrieve Router
    let retrieve_router = Arc::new(retrieval::retrieve::Router::new(
        Arc::new(retrieval::retrieve::github::GithubClient::new(
            github_client.clone(),
        )),
        Arc::new(retrieval::retrieve::filesystem::FilesystemClient::new(
            filesystem_client.clone(),
        )),
        Arc::new(retrieval::retrieve::mock::MockClient),
    ));

    // MCP Client
    let mcp_client = Arc::new(objectiveai_sdk::mcp::Client::new(
        http_client.clone(),
        user_agent.clone(),
        x_title.clone(),
        http_referer.clone(),
        std::time::Duration::from_millis(mcp_connect_timeout),
        std::time::Duration::from_millis(
            mcp_backoff_current_interval,
        ),
        std::time::Duration::from_millis(
            mcp_backoff_initial_interval,
        ),
        mcp_backoff_randomization_factor,
        mcp_backoff_multiplier,
        std::time::Duration::from_millis(mcp_backoff_max_interval),
        std::time::Duration::from_millis(
            mcp_backoff_max_elapsed_time,
        ),
        std::time::Duration::from_millis(mcp_call_timeout),
    ));

    // Lazy in-process mcp-proxy. Each per-agent MCP connection goes
    // through this; it boots on the first request that needs it and
    // lives for the rest of the program.
    //
    // Propagate the api's loaded MCP config into the in-process proxy's
    // ConfigBuilder so the proxy honours the same env vars
    // (`MCP_CONNECT_TIMEOUT`, `MCP_CALL_TIMEOUT`, `MCP_BACKOFF_*`) the
    // api itself reads — without this the proxy would fall back to its
    // own crate-internal defaults.
    let proxy_encryption_key: Option<[u8; 32]> = mcp_encryption_key
        .as_deref()
        .and_then(|s| match objectiveai_mcp_proxy::parse_key_env(s) {
            Ok(opt) => opt,
            Err(e) => {
                eprintln!("MCP_ENCRYPTION_KEY parse failed; falling back to ephemeral key in proxy: {e}");
                None
            }
        });
    let proxy_spawner = Arc::new(agent::completions::ProxySpawner::new(move || {
        objectiveai_mcp_proxy::ConfigBuilder {
            mcp_connect_timeout: Some(mcp_connect_timeout),
            mcp_call_timeout: Some(mcp_call_timeout),
            mcp_backoff_current_interval: Some(mcp_backoff_current_interval),
            mcp_backoff_initial_interval: Some(mcp_backoff_initial_interval),
            mcp_backoff_randomization_factor: Some(mcp_backoff_randomization_factor),
            mcp_backoff_multiplier: Some(mcp_backoff_multiplier),
            mcp_backoff_max_interval: Some(mcp_backoff_max_interval),
            mcp_backoff_max_elapsed_time: Some(mcp_backoff_max_elapsed_time),
            mcp_encryption_key: proxy_encryption_key,
            ..Default::default()
        }
    }));

    // Agent Completions Client
    let agent_completions_client = Arc::new(agent::completions::Client::new(
        mcp_client.clone(),
        proxy_spawner,
        mcp_authorization.clone(),
        retrieve_router.clone(),
        Arc::new(agent::completions::usage_handler::LogUsageHandler),
        Arc::new(agent::completions::openrouter::Client::new(
            http_client.clone(),
            openrouter_address,
            openrouter_authorization,
            user_agent.clone(),
            x_title.clone(),
            http_referer.clone(),
        )),
        Arc::new(agent::completions::claude_agent_sdk::Client::new(user_agent.clone(), claude_agent_sdk_enabled, claude_agent_sdk_rate_limit_max_retries, claude_agent_sdk_rate_limit_max_wait_secs, claude_agent_sdk_query_limit)),
        Arc::new(agent::completions::codex_sdk::Client::new(user_agent, codex_sdk_enabled, codex_sdk_rate_limit_max_retries, codex_sdk_rate_limit_max_wait_secs, codex_sdk_query_limit, http_client)),
        Arc::new(agent::completions::mock::Client {
            delay: std::time::Duration::from_millis(mock_delay_ms),
            max_tool_calls: mock_max_tool_calls,
        }),
        std::time::Duration::from_millis(
            agent_completions_backoff_current_interval,
        ),
        std::time::Duration::from_millis(
            agent_completions_backoff_initial_interval,
        ),
        agent_completions_backoff_randomization_factor,
        agent_completions_backoff_multiplier,
        std::time::Duration::from_millis(agent_completions_backoff_max_interval),
        std::time::Duration::from_millis(
            agent_completions_backoff_max_elapsed_time,
        ),
        std::time::Duration::from_millis(agent_completions_first_chunk_timeout),
        std::time::Duration::from_millis(agent_completions_other_chunk_timeout),
    ));

    // Reverse-channel registry for the objectiveai-MCP endpoint. WS
    // handlers populate this on upgrade; the MCP endpoint route reads
    // it when a proxy upstream dials in for a session.
    let reverse_channels = streaming_ws::new_reverse_channel_registry();
    // SSE listener registry: per-(response_id, McpKind) broadcast
    // feeding the MCP GET notifications stream. The conduit WS recv
    // loop publishes here when the CLI pushes `McpListChanged`; the
    // GET handler subscribes from here.
    let mcp_listeners = crate::objectiveai_mcp::McpListenerRegistry::new();
    // Public + loopback-MCP listeners bound in parallel. Both
    // listeners need to be up before the process can serve a
    // request that touches `client_objectiveai_mcp`, and neither
    // bind blocks the other — `try_join` shaves the second bind's
    // syscall latency off cold start (matters on Cloud Run where
    // boot time bills + counts toward request latency).
    //
    // The MCP listener binds `127.0.0.1` so the kernel rejects any
    // non-loopback dialer outright — the proxy running inside the
    // API process is the only intended caller, and it always dials
    // over loopback. Ephemeral port keeps the binding cheap and
    // conflict-free; we read it back below and stamp it onto
    // `ReverseAttachConfig.mcp_port` so the agent client can
    // synthesize the matching `http://127.0.0.1:<port>/objectiveai-
    // mcp` URL on every per-agent `X-MCP-Servers` header.
    let (listener, mcp_listener) = tokio::try_join!(
        tokio::net::TcpListener::bind(format!("{}:{}", address, port)),
        tokio::net::TcpListener::bind(("127.0.0.1", 0u16)),
    )?;
    let mcp_port = mcp_listener.local_addr()?.port();

    let reverse_attach = streaming_ws::ReverseAttachConfig {
        registry: reverse_channels.clone(),
        mcp_port,
        mcp_listeners: mcp_listeners.clone(),
    };

    // Vector Completions Client
    let vector_completions_client = Arc::new(vector::completions::Client::new(
        agent_completions_client.clone(),
        retrieve_router.clone(),
        completion_votes_fetcher.clone(),
        cache_vote_fetcher.clone(),
        Arc::new(vector::completions::usage_handler::LogUsageHandler),
    ));

    // Vector Completions Cache Client
    let vector_completions_cache_client =
        Arc::new(vector::completions::cache::Client::new(
            completion_votes_fetcher.clone(),
            cache_vote_fetcher.clone(),
        ));

    // Retrieval: List Router
    let list_router = Arc::new(retrieval::list::Router::new(
        Arc::new(retrieval::list::objectiveai::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        )),
        Arc::new(retrieval::list::filesystem::FilesystemClient::new(
            filesystem_client.clone(),
        )),
        Arc::new(retrieval::list::mock::MockClient),
    ));

    // Retrieval: Usage Router
    let usage_router = Arc::new(retrieval::usage::Router::new(
        Arc::new(retrieval::usage::objectiveai::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        )),
    ));

    // Function Executions Client
    let function_executions_client =
        Arc::new(functions::executions::Client::new(
            agent_completions_client.clone(),
            vector_completions_client.clone(),
            retrieve_router.clone(),
            Arc::new(functions::executions::usage_handler::LogUsageHandler),
        ));

    // Functions Profiles Computations Client
    let profile_computations_client =
        Arc::new(functions::profiles::computations::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        ));

    // Auth Client
    let auth_client = Arc::new(auth::ObjectiveAiClient::new(
        objectiveai_http_client.clone(),
    ));

    // Persistent Cache Client
    #[cfg(feature = "sqlite-persistent-cache")]
    let persistent_cache = Arc::new(
        ctx::persistent_cache::sqlite::SqlitePersistentCacheClient::new(
            config_base_dir,
            std::time::Duration::from_millis(persistent_cache_transient_ttl_ms),
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
    );
    #[cfg(not(feature = "sqlite-persistent-cache"))]
    let persistent_cache = {
        let _ = persistent_cache_transient_ttl_ms;
        let _ = &config_base_dir;
        Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient)
    };

    // Router
    let app = axum::Router::new()
        // Agent Completions - create (transport selected by X-Transport header)
        .route(
            "/agent/completions",
            axum::routing::any({
                let agent_completions_client = agent_completions_client.clone();
                let persistent_cache = persistent_cache.clone();
                let reverse_attach = reverse_attach.clone();
                move |transport: streaming_ws::Transport, req: axum::extract::Request| {
                    let agent_completions_client = agent_completions_client.clone();
                    let persistent_cache = persistent_cache.clone();
                    let reverse_attach = reverse_attach.clone();
                    async move {
                        use axum::extract::FromRequest;
                        use axum::extract::FromRequestParts;
                        let (mut parts, body) = req.into_parts();
                        let headers = parts.headers.clone();
                        match transport {
                            streaming_ws::Transport::Sse => {
                                let req = axum::extract::Request::from_parts(parts, body);
                                match Json::<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>::from_request(req, &()).await {
                                    Ok(Json(body)) => create_agent_completion(agent_completions_client, headers, persistent_cache, suppress_output, body).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                            streaming_ws::Transport::WebSocket => {
                                match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
                                    Ok(ws) => streaming_ws_handlers::create_agent_completion_ws(agent_completions_client, reverse_attach, headers, persistent_cache, suppress_output, ws).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                        }
                    }
                }
            }),
        )
        // Vector Completions - create (transport selected by X-Transport header)
        .route(
            "/vector/completions",
            axum::routing::any({
                let vector_completions_client = vector_completions_client.clone();
                let agent_completions_client = agent_completions_client.clone();
                let persistent_cache = persistent_cache.clone();
                let reverse_attach = reverse_attach.clone();
                move |transport: streaming_ws::Transport, req: axum::extract::Request| {
                    let vector_completions_client = vector_completions_client.clone();
                    let agent_completions_client = agent_completions_client.clone();
                    let persistent_cache = persistent_cache.clone();
                    let reverse_attach = reverse_attach.clone();
                    async move {
                        use axum::extract::FromRequest;
                        use axum::extract::FromRequestParts;
                        let (mut parts, body) = req.into_parts();
                        let headers = parts.headers.clone();
                        match transport {
                            streaming_ws::Transport::Sse => {
                                let req = axum::extract::Request::from_parts(parts, body);
                                match Json::<objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams>::from_request(req, &()).await {
                                    Ok(Json(body)) => create_vector_completion(vector_completions_client, headers, persistent_cache, suppress_output, body).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                            streaming_ws::Transport::WebSocket => {
                                match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
                                    Ok(ws) => streaming_ws_handlers::create_vector_completion_ws(vector_completions_client, agent_completions_client, reverse_attach, headers, persistent_cache, suppress_output, ws).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                        }
                    }
                }
            }),
        )
        // Vector Completions - get completion votes
        .route(
            "/vector/completions/votes",
            axum::routing::post({
                let vector_completions_cache_client =
                    vector_completions_cache_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai_sdk::vector::completions::cache::request::GetCompletionVotesRequest,
                >| {
                    get_vector_completion_votes(
                        vector_completions_cache_client,
                        headers,
                        persistent_cache,
                        suppress_output,
                        body,
                    )
                }
            }),
        )
        // Vector Completions - get cache vote
        .route(
            "/vector/completions/cache",
            axum::routing::post({
                let vector_completions_cache_client =
                    vector_completions_cache_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai_sdk::vector::completions::cache::request::CacheVoteRequestOwned,
                >| {
                    get_vector_cache_vote(
                        vector_completions_cache_client,
                        headers,
                        persistent_cache,
                        suppress_output,
                        body,
                    )
                }
            }),
        )
        // Functions - list
        .route(
            "/functions/list",
            axum::routing::post({
                let list_router = list_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::functions::request::ListFunctionsRequest,
                >| {
                    list_functions(list_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Functions - get
        .route(
            "/functions",
            axum::routing::post({
                let retrieve_router = retrieve_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::RemotePathCommitOptional,
                >| {
                    get_function(retrieve_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Functions - get usage
        .route(
            "/functions/usage",
            axum::routing::post({
                let usage_router = usage_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::functions::request::GetFunctionRequest,
                >| {
                    get_function_usage(usage_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Function Executions - create (transport selected by X-Transport header)
        .route(
            "/functions/executions",
            axum::routing::any({
                let function_executions_client = function_executions_client.clone();
                let agent_completions_client = agent_completions_client.clone();
                let persistent_cache = persistent_cache.clone();
                let reverse_attach = reverse_attach.clone();
                move |transport: streaming_ws::Transport, req: axum::extract::Request| {
                    let function_executions_client = function_executions_client.clone();
                    let agent_completions_client = agent_completions_client.clone();
                    let persistent_cache = persistent_cache.clone();
                    let reverse_attach = reverse_attach.clone();
                    async move {
                        use axum::extract::FromRequest;
                        use axum::extract::FromRequestParts;
                        let (mut parts, body) = req.into_parts();
                        let headers = parts.headers.clone();
                        match transport {
                            streaming_ws::Transport::Sse => {
                                let req = axum::extract::Request::from_parts(parts, body);
                                match Json::<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>::from_request(req, &()).await {
                                    Ok(Json(body)) => execute_function(function_executions_client, headers, persistent_cache, suppress_output, body).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                            streaming_ws::Transport::WebSocket => {
                                match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
                                    Ok(ws) => streaming_ws_handlers::execute_function_ws(function_executions_client, agent_completions_client, reverse_attach, headers, persistent_cache, suppress_output, ws).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                        }
                    }
                }
            }),
        )
        // Function Profiles - list
        .route(
            "/functions/profiles/list",
            axum::routing::post({
                let list_router = list_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::functions::profiles::request::ListProfilesRequest,
                >| {
                    list_profiles(list_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Function Profiles - get
        .route(
            "/functions/profiles",
            axum::routing::post({
                let retrieve_router = retrieve_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::RemotePathCommitOptional,
                >| {
                    get_profile(retrieve_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Function Profiles - get usage
        .route(
            "/functions/profiles/usage",
            axum::routing::post({
                let usage_router = usage_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::functions::profiles::request::GetProfileRequest,
                >| {
                    get_profile_usage(usage_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Function-Profile Pairs - list
        .route(
            "/functions/profiles/pairs/list",
            axum::routing::post({
                let list_router = list_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::functions::request::ListFunctionProfilePairsRequest,
                >| {
                    list_function_profile_pairs(list_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Function-Profile Pairs - get usage
        .route(
            "/functions/profiles/pairs/usage",
            axum::routing::post({
                let usage_router = usage_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::functions::request::GetFunctionProfilePairUsageRequest,
                >| {
                    get_function_profile_pair_usage(usage_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Function Profile Computations - create (transport selected by X-Transport header)
        .route(
            "/functions/profiles/compute",
            axum::routing::any({
                let profile_computations_client =
                    profile_computations_client.clone();
                let agent_completions_client = agent_completions_client.clone();
                let persistent_cache = persistent_cache.clone();
                let reverse_attach = reverse_attach.clone();
                move |transport: streaming_ws::Transport, req: axum::extract::Request| {
                    let profile_computations_client = profile_computations_client.clone();
                    let agent_completions_client = agent_completions_client.clone();
                    let persistent_cache = persistent_cache.clone();
                    let reverse_attach = reverse_attach.clone();
                    async move {
                        use axum::extract::FromRequest;
                        use axum::extract::FromRequestParts;
                        let (mut parts, body) = req.into_parts();
                        let headers = parts.headers.clone();
                        match transport {
                            streaming_ws::Transport::Sse => {
                                let req = axum::extract::Request::from_parts(parts, body);
                                match Json::<objectiveai_sdk::functions::profiles::computations::request::FunctionProfileComputationCreateParams>::from_request(req, &()).await {
                                    Ok(Json(body)) => create_profile_computation(profile_computations_client, headers, persistent_cache, suppress_output, body).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                            streaming_ws::Transport::WebSocket => {
                                match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
                                    Ok(ws) => streaming_ws_handlers::create_profile_computation_ws(profile_computations_client, agent_completions_client, reverse_attach, headers, persistent_cache, suppress_output, ws).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                        }
                    }
                }
            }),
        )
        // Auth - create API key
        .route(
            "/auth/keys",
            axum::routing::post({
                let auth_client = auth_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai_sdk::auth::request::CreateApiKeyRequest,
                >| {
                    create_api_key(auth_client, headers, persistent_cache, suppress_output, body)
                }
            }),
        )
        // Auth - create OpenRouter BYOK API key
        .route(
            "/auth/keys/openrouter",
            axum::routing::post({
                let auth_client = auth_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai_sdk::auth::request::CreateOpenRouterByokApiKeyRequest,
                >| {
                    create_openrouter_byok_api_key(auth_client, headers, persistent_cache, suppress_output, body)
                }
            }),
        )
        // Auth - disable API key
        .route(
            "/auth/keys",
            axum::routing::delete({
                let auth_client = auth_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai_sdk::auth::request::DisableApiKeyRequest,
                >| {
                    disable_api_key(auth_client, headers, persistent_cache, suppress_output, body)
                }
            }),
        )
        // Auth - delete OpenRouter BYOK API key
        .route(
            "/auth/keys/openrouter",
            axum::routing::delete({
                let auth_client = auth_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap| {
                    delete_openrouter_byok_api_key(auth_client, headers, persistent_cache, suppress_output)
                }
            }),
        )
        // Auth - list API keys
        .route(
            "/auth/keys",
            axum::routing::get({
                let auth_client = auth_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap| {
                    list_api_keys(auth_client, headers, persistent_cache, suppress_output)
                }
            }),
        )
        // Auth - get OpenRouter BYOK API key
        .route(
            "/auth/keys/openrouter",
            axum::routing::get({
                let auth_client = auth_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap| {
                    get_openrouter_byok_api_key(auth_client, headers, persistent_cache, suppress_output)
                }
            }),
        )
        // Auth - get credits
        .route(
            "/auth/credits",
            axum::routing::get({
                let auth_client = auth_client.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap| {
                    get_credits(auth_client, headers, persistent_cache, suppress_output)
                }
            }),
        )
        // Swarm - list
        .route(
            "/swarms/list",
            axum::routing::post({
                let list_router = list_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::swarm::request::ListSwarmsRequest,
                >| {
                    list_swarms(list_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Swarm - get
        .route(
            "/swarms",
            axum::routing::post({
                let retrieve_router = retrieve_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::RemotePathCommitOptional,
                >| {
                    get_swarm(retrieve_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Swarm - get usage
        .route(
            "/swarms/usage",
            axum::routing::post({
                let usage_router = usage_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::swarm::request::GetSwarmRequest,
                >| {
                    get_swarm_usage(usage_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Agent - list
        .route(
            "/agents/list",
            axum::routing::post({
                let list_router = list_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::agent::request::ListAgentsRequest,
                >| {
                    list_agents(list_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Agent - get
        .route(
            "/agents",
            axum::routing::post({
                let retrieve_router = retrieve_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::RemotePathCommitOptional,
                >| {
                    get_agent(retrieve_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Agent - get usage
        .route(
            "/agents/usage",
            axum::routing::post({
                let usage_router = usage_router.clone();
                let persistent_cache = persistent_cache.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai_sdk::agent::request::GetAgentRequest,
                >| {
                    get_agent_usage(usage_router, headers, persistent_cache, suppress_output, params)
                }
            }),
        )
        // Error - create (transport selected by X-Transport header)
        .route(
            "/error",
            axum::routing::any({
                let error_client = Arc::new(crate::error::Client::new());
                let agent_completions_client = agent_completions_client.clone();
                let persistent_cache = persistent_cache.clone();
                let reverse_attach = reverse_attach.clone();
                move |transport: streaming_ws::Transport, req: axum::extract::Request| {
                    let error_client = error_client.clone();
                    let agent_completions_client = agent_completions_client.clone();
                    let persistent_cache = persistent_cache.clone();
                    let reverse_attach = reverse_attach.clone();
                    async move {
                        use axum::extract::FromRequest;
                        use axum::extract::FromRequestParts;
                        let (mut parts, body) = req.into_parts();
                        let headers = parts.headers.clone();
                        match transport {
                            streaming_ws::Transport::Sse => {
                                let req = axum::extract::Request::from_parts(parts, body);
                                match Json::<objectiveai_sdk::error::request::ErrorCreateParams>::from_request(req, &()).await {
                                    Ok(Json(body)) => create_error(error_client, headers, persistent_cache, suppress_output, body).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                            streaming_ws::Transport::WebSocket => {
                                match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
                                    Ok(ws) => streaming_ws_handlers::create_error_ws(error_client, agent_completions_client, reverse_attach, headers, persistent_cache, suppress_output, ws).await,
                                    Err(rej) => rej.into_response(),
                                }
                            }
                        }
                    }
                }
            }),
        )
        // CORS
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
                .expose_headers(tower_http::cors::Any),
        );

    // ObjectiveAI-MCP server — Streamable HTTP MCP + the `/notify`
    // extensions. Six routes total (POST/GET/DELETE on the root,
    // POST/GET on `/notify`, GET on `/notify/queued`). Lives on its
    // own loopback-only listener (`mcp_listener` above) so non-
    // loopback callers physically cannot reach it. No CORS layer —
    // there's nothing cross-origin about loopback-to-loopback. See
    // `objectiveai_mcp::router`.
    let mcp_app = axum::Router::new().merge(crate::objectiveai_mcp::router(
        reverse_channels.clone(),
        mcp_listeners.clone(),
    ));

    Ok((listener, app, mcp_listener, mcp_app))
}

pub async fn serve(listener: tokio::net::TcpListener, app: axum::Router) -> std::io::Result<()> {
    axum::serve(listener, app).await
}

pub async fn run(config: Config) -> std::io::Result<()> {
    let suppress_output = config.suppress_output;
    let objectiveai_dir = config.objectiveai_dir.clone();
    let (listener, app, mcp_listener, mcp_app) = setup(config).await?;

    // There is only ever ONE api server per OBJECTIVEAI_DIR: claim
    // key "api" in <dir>/bin/locks the moment the listen address is
    // known, publishing the URL clients connect with (wildcard binds
    // map to loopback). Anyone can lockfile::read it without owning
    // the lock; the claim itself is held until process death
    // (LockClaim leaks on drop by design) and the kernel releases it
    // on any exit, crash included.
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
    if objectiveai_sdk::lockfile::try_acquire(
        &objectiveai_dir.join("bin").join("locks"),
        "api",
        &connect_url,
    )
    .await
    .is_none()
    {
        return Err(std::io::Error::other(
            "another objectiveai-api instance already holds the api lock for this OBJECTIVEAI_DIR",
        ));
    }

    if !suppress_output {
        let mcp_addr = mcp_listener.local_addr()?;
        eprintln!("listening on {addr}");
        eprintln!("mcp listening on {mcp_addr} (loopback only)");
    }
    // Public + loopback-MCP listeners served concurrently. On Cloud
    // Run there is no infra benefit to staggering them — the
    // container needs both up before it can serve a single request
    // that touches `client_objectiveai_mcp` — so we `try_join` to
    // bring them up in parallel and tear the process down the
    // moment either listener's accept loop errors.
    tokio::try_join!(serve(listener, app), serve(mcp_listener, mcp_app))?;
    Ok(())
}

// Create Context

pub(crate) fn context(headers: &axum::http::HeaderMap, persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>, suppress_output: bool) -> ctx::Context<ctx::DefaultContextExt, impl ctx::persistent_cache::PersistentCacheClient> {
    ctx::Context::new(
        Arc::new(ctx::DefaultContextExt),
        persistent_cache,
        rust_decimal::Decimal::ONE,
        suppress_output,
        headers,
    )
}

// Agent Completions

async fn create_agent_completion(
    client: Arc<
        agent::completions::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl agent::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    body: objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    if body.stream.unwrap_or(false) {
        match client
            .create_streaming_handle_usage(
                ctx,
                Arc::new(body),
                None,
                None, // disable_tools
                vec![], // extra_mcp_servers
                indexmap::IndexMap::new(), // extra_mcp_headers
                None,
            )
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .filter_map(|item| {
                        match item {
                            agent::completions::StreamItem::Chunk(chunk) => {
                                Some(Ok::<Event, Infallible>(
                                    Event::default()
                                        .data(serde_json::to_string(&chunk).unwrap()),
                                ))
                            }
                            agent::completions::StreamItem::State(_) => None,
                        }
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client
            .create_unary_handle_usage(
                ctx,
                Arc::new(body),
                None,
                None, // disable_tools
                vec![], // extra_mcp_servers
                indexmap::IndexMap::new(), // extra_mcp_headers
                None,
            )
            .await
        {
            Ok(r) => Json(r).into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    }
}

// Vector Completions

async fn create_vector_completion(
    client: Arc<
        vector::completions::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl agent::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::completion_votes_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    body: objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    if body.stream.unwrap_or(false) {
        match client
            .create_streaming_handle_usage(ctx, Arc::new(body))
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .map(|chunk| {
                        Ok::<Event, Infallible>(
                            Event::default()
                                .data(serde_json::to_string(&chunk).unwrap()),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client.create_unary_handle_usage(ctx, Arc::new(body)).await {
            Ok(r) => Json(r).into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    }
}

// Functions

async fn list_functions(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::functions::request::ListFunctionsRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    let source = params.source.map(|s| match s {
        objectiveai_sdk::functions::request::ListFunctionsSource::All => retrieval::list::SourceFilter::All,
        objectiveai_sdk::functions::request::ListFunctionsSource::Mock => retrieval::list::SourceFilter::Mock,
        objectiveai_sdk::functions::request::ListFunctionsSource::Filesystem => retrieval::list::SourceFilter::Filesystem,
        objectiveai_sdk::functions::request::ListFunctionsSource::Objectiveai => retrieval::list::SourceFilter::Objectiveai,
    });
    match list_router.list_functions(&ctx, source).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_function_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::functions::request::GetFunctionRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match usage_router.get_function_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn execute_function(
    client: Arc<
        functions::executions::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::completion_votes_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::executions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    request: objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    if request.stream.unwrap_or(false) {
        match client
            .create_streaming_handle_usage(ctx, Arc::new(request))
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .map(|chunk| {
                        Ok::<Event, Infallible>(
                            Event::default()
                                .data(serde_json::to_string(&chunk).unwrap()),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client
            .create_unary_handle_usage(ctx, Arc::new(request))
            .await
        {
            Ok(r) => Json(r).into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    }
}

// Profiles

async fn list_profiles(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::functions::profiles::request::ListProfilesRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    let source = params.source.map(|s| match s {
        objectiveai_sdk::functions::profiles::request::ListProfilesSource::All => retrieval::list::SourceFilter::All,
        objectiveai_sdk::functions::profiles::request::ListProfilesSource::Mock => retrieval::list::SourceFilter::Mock,
        objectiveai_sdk::functions::profiles::request::ListProfilesSource::Filesystem => retrieval::list::SourceFilter::Filesystem,
        objectiveai_sdk::functions::profiles::request::ListProfilesSource::Objectiveai => retrieval::list::SourceFilter::Objectiveai,
    });
    match list_router.list_profiles(&ctx, source).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_profile_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::functions::profiles::request::GetProfileRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match usage_router.get_profile_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Function-Profile Pairs

async fn list_function_profile_pairs(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    _params: objectiveai_sdk::functions::request::ListFunctionProfilePairsRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match list_router.list_function_profile_pairs(&ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_function_profile_pair_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::functions::request::GetFunctionProfilePairUsageRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match usage_router.get_function_profile_pair_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Vector Completions Cache

async fn get_vector_completion_votes(
    client: Arc<
        vector::completions::cache::Client<
            ctx::DefaultContextExt,
            impl vector::completions::completion_votes_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    body: objectiveai_sdk::vector::completions::cache::request::GetCompletionVotesRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client.fetch_completion_votes(ctx, &body.id).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_vector_cache_vote(
    client: Arc<
        vector::completions::cache::Client<
            ctx::DefaultContextExt,
            impl vector::completions::completion_votes_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    body: objectiveai_sdk::vector::completions::cache::request::CacheVoteRequestOwned,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client
        .fetch_cache_vote(
            ctx,
            &body.agent,
            &body.messages,
            &body.responses,
        )
        .await
    {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Functions - get

async fn get_function(
    retrieve_router: Arc<RetrieveRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::RemotePathCommitOptional,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match retrieve_router.endpoint_get_function(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Profiles - get

async fn get_profile(
    retrieve_router: Arc<RetrieveRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::RemotePathCommitOptional,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match retrieve_router.endpoint_get_profile(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Profile Computations

async fn create_profile_computation(
    // client: Arc<
    //     impl functions::profiles::computations::Client<ctx::DefaultContextExt>
    //     + Send
    //     + Sync
    //     + 'static,
    // >,
    // https://github.com/rust-lang/rust/issues/100013
    // using a concrete type for client instead
    client: Arc<functions::profiles::computations::ObjectiveAiClient>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    request: objectiveai_sdk::functions::profiles::computations::request::FunctionProfileComputationCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    if request.stream.unwrap_or(false) {
        match client.create_streaming(ctx, Arc::new(request)).await {
            Ok(stream) => Sse::new(
                stream
                    .map(|result| {
                        Ok::<Event, Infallible>(
                            Event::default().data(
                                match result {
                                    Ok(chunk) => serde_json::to_string(&chunk),
                                    Err(e) => serde_json::to_string(&e),
                                }
                                .unwrap(),
                            ),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => e.into_response(),
        }
    } else {
        match client.create_unary(ctx, Arc::new(request)).await {
            Ok(r) => Json(r).into_response(),
            Err(e) => e.into_response(),
        }
    }
}

// Auth

async fn create_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    body: objectiveai_sdk::auth::request::CreateApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client.create_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn create_openrouter_byok_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    body: objectiveai_sdk::auth::request::CreateOpenRouterByokApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client.create_openrouter_byok_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn disable_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    body: objectiveai_sdk::auth::request::DisableApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client.disable_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn delete_openrouter_byok_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client.delete_openrouter_byok_api_key(ctx).await {
        Ok(()) => axum::http::StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn list_api_keys(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client.list_api_keys(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_openrouter_byok_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client.get_openrouter_byok_api_key(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_credits(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match client.get_credits(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Swarm

async fn list_swarms(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::swarm::request::ListSwarmsRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    let source = params.source.map(|s| match s {
        objectiveai_sdk::swarm::request::ListSwarmsSource::All => retrieval::list::SourceFilter::All,
        objectiveai_sdk::swarm::request::ListSwarmsSource::Mock => retrieval::list::SourceFilter::Mock,
        objectiveai_sdk::swarm::request::ListSwarmsSource::Filesystem => retrieval::list::SourceFilter::Filesystem,
        objectiveai_sdk::swarm::request::ListSwarmsSource::Objectiveai => retrieval::list::SourceFilter::Objectiveai,
    });
    match list_router.list_swarms(&ctx, source).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_swarm(
    retrieve_router: Arc<RetrieveRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::RemotePathCommitOptional,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match retrieve_router.endpoint_get_swarm(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_swarm_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::swarm::request::GetSwarmRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match usage_router.get_swarm_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Agent

async fn list_agents(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::agent::request::ListAgentsRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    let source = params.source.map(|s| match s {
        objectiveai_sdk::agent::request::ListAgentsSource::All => retrieval::list::SourceFilter::All,
        objectiveai_sdk::agent::request::ListAgentsSource::Mock => retrieval::list::SourceFilter::Mock,
        objectiveai_sdk::agent::request::ListAgentsSource::Filesystem => retrieval::list::SourceFilter::Filesystem,
        objectiveai_sdk::agent::request::ListAgentsSource::Objectiveai => retrieval::list::SourceFilter::Objectiveai,
    });
    match list_router.list_agents(&ctx, source).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_agent(
    retrieve_router: Arc<RetrieveRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::RemotePathCommitOptional,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match retrieve_router.endpoint_get_agent(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_agent_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    params: objectiveai_sdk::agent::request::GetAgentRequest,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    match usage_router.get_agent_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}
// Error

async fn create_error(
    client: Arc<crate::error::Client>,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
    suppress_output: bool,
    body: objectiveai_sdk::error::request::ErrorCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers, persistent_cache, suppress_output);
    if body.stream.unwrap_or(false) {
        match client.create_streaming(&ctx, &body) {
            Ok(stream) => Sse::new(
                stream
                    .map(|result| {
                        Ok::<Event, Infallible>(
                            Event::default().data(
                                match result {
                                    Ok(chunk) => serde_json::to_string(&chunk),
                                    Err(e) => serde_json::to_string(&e),
                                }
                                .unwrap(),
                            ),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => e.into_response(),
        }
    } else {
        match client.create_unary(&ctx, &body) {
            Ok(r) => Json(r).into_response(),
            Err(e) => e.into_response(),
        }
    }
}
