pub mod config;
pub mod address;
pub mod port;
pub mod claude_agent_sdk;
pub mod codex_sdk;
pub mod objectiveai_authorization;
pub mod openrouter_authorization;
pub mod github_authorization;
pub mod mcp_authorization;
pub mod user_agent;
pub mod http_referer;
pub mod x_title;
pub mod commit_author_name;
pub mod commit_author_email;
pub mod spawn;
pub mod kill;
mod run;
pub mod detach;
pub mod conduit;

// Shared infrastructure for per-endpoint subcommands.
pub mod agent_id_arg;
pub mod body;
pub mod client;
pub mod call;

// One subtree per URL root segment. Every leaf inside these mirrors a
// route in `objectiveai-api/src/run.rs` 1:1 — see
// `tests/api_endpoint_coverage.rs`.
pub mod agent;
pub mod vector;
pub mod functions;
pub mod auth;
pub mod swarms;
pub mod agents;
pub mod error;
pub mod laboratories;

pub use run::*;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// API configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Local API server bind address
    Address {
        #[command(subcommand)]
        command: address::Commands,
    },
    /// Local API server bind port
    Port {
        #[command(subcommand)]
        command: port::Commands,
    },
    /// Claude Agent SDK enabled
    ClaudeAgentSdk {
        #[command(subcommand)]
        command: claude_agent_sdk::Commands,
    },
    /// Codex SDK enabled
    CodexSdk {
        #[command(subcommand)]
        command: codex_sdk::Commands,
    },
    /// `Authorization` header
    ObjectiveaiAuthorization {
        #[command(subcommand)]
        command: objectiveai_authorization::Commands,
    },
    /// `X-OPENROUTER-AUTHORIZATION` header
    OpenrouterAuthorization {
        #[command(subcommand)]
        command: openrouter_authorization::Commands,
    },
    /// `X-GITHUB-AUTHORIZATION` header
    GithubAuthorization {
        #[command(subcommand)]
        command: github_authorization::Commands,
    },
    /// `X-MCP-AUTHORIZATION` header (key/value map)
    McpAuthorization {
        #[command(subcommand)]
        command: mcp_authorization::Commands,
    },
    /// `User-Agent` header
    UserAgent {
        #[command(subcommand)]
        command: user_agent::Commands,
    },
    /// `Referer` / `HTTP-Referer` header
    HttpReferer {
        #[command(subcommand)]
        command: http_referer::Commands,
    },
    /// `X-Title` header
    XTitle {
        #[command(subcommand)]
        command: x_title::Commands,
    },
    /// `X-COMMIT-AUTHOR-NAME` header
    CommitAuthorName {
        #[command(subcommand)]
        command: commit_author_name::Commands,
    },
    /// `X-COMMIT-AUTHOR-EMAIL` header
    CommitAuthorEmail {
        #[command(subcommand)]
        command: commit_author_email::Commands,
    },
    /// `/agent/*` endpoints
    Agent {
        #[command(subcommand)]
        command: agent::Commands,
    },
    /// `/vector/*` endpoints
    Vector {
        #[command(subcommand)]
        command: vector::Commands,
    },
    /// `/functions/*` endpoints
    Functions {
        #[command(subcommand)]
        command: functions::Commands,
    },
    /// `/auth/*` endpoints
    Auth {
        #[command(subcommand)]
        command: auth::Commands,
    },
    /// `/swarms/*` endpoints
    Swarms {
        #[command(subcommand)]
        command: swarms::Commands,
    },
    /// `/agents/*` endpoints
    Agents {
        #[command(subcommand)]
        command: agents::Commands,
    },
    /// `/error` endpoint
    Error {
        #[command(subcommand)]
        command: error::Commands,
    },
    /// `/laboratories/*` endpoints
    Laboratories {
        #[command(subcommand)]
        command: laboratories::Commands,
    },
    /// Spawn the `objectiveai-api` server in the background.
    /// Errors if it's already running.
    Spawn,
    /// Terminate every running `objectiveai-api` process.
    /// Idempotent — succeeds with count = 0 if none were running.
    Kill,
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config, handle).await,
            Commands::Address { command } => command.handle(cli_config, handle).await,
            Commands::Port { command } => command.handle(cli_config, handle).await,
            Commands::ClaudeAgentSdk { command } => command.handle(cli_config, handle).await,
            Commands::CodexSdk { command } => command.handle(cli_config, handle).await,
            Commands::ObjectiveaiAuthorization { command } => command.handle(cli_config, handle).await,
            Commands::OpenrouterAuthorization { command } => command.handle(cli_config, handle).await,
            Commands::GithubAuthorization { command } => command.handle(cli_config, handle).await,
            Commands::McpAuthorization { command } => command.handle(cli_config, handle).await,
            Commands::UserAgent { command } => command.handle(cli_config, handle).await,
            Commands::HttpReferer { command } => command.handle(cli_config, handle).await,
            Commands::XTitle { command } => command.handle(cli_config, handle).await,
            Commands::CommitAuthorName { command } => command.handle(cli_config, handle).await,
            Commands::CommitAuthorEmail { command } => command.handle(cli_config, handle).await,
            Commands::Agent { command } => command.handle(cli_config, handle).await,
            Commands::Vector { command } => command.handle(cli_config, handle).await,
            Commands::Functions { command } => command.handle(cli_config, handle).await,
            Commands::Auth { command } => command.handle(cli_config, handle).await,
            Commands::Swarms { command } => command.handle(cli_config, handle).await,
            Commands::Agents { command } => command.handle(cli_config, handle).await,
            Commands::Error { command } => command.handle(cli_config, handle).await,
            Commands::Laboratories { command } => command.handle(cli_config, handle).await,
            Commands::Spawn => spawn::handle(cli_config, handle).await,
            Commands::Kill => kill::handle(cli_config, handle).await,
        }
    }
}
