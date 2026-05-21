pub mod config;
pub mod address;
pub mod port;
pub mod spawn;
pub mod kill;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// MCP configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Local MCP server bind address
    Address {
        #[command(subcommand)]
        command: address::Commands,
    },
    /// Local MCP server bind port
    Port {
        #[command(subcommand)]
        command: port::Commands,
    },
    /// Spawn the `objectiveai-mcp` server in the background.
    /// Errors if it's already running.
    Spawn,
    /// Terminate every running `objectiveai-mcp` process.
    /// Idempotent — succeeds with count = 0 if none were running.
    Kill,
}

impl Commands {
    pub async fn handle(
        self,
        cli_config: &crate::Config,
        handle: &objectiveai_sdk::cli::output::Handle,
    ) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config, handle).await,
            Commands::Address { command } => command.handle(cli_config, handle).await,
            Commands::Port { command } => command.handle(cli_config, handle).await,
            Commands::Spawn => spawn::handle(cli_config, handle).await,
            Commands::Kill => kill::handle(cli_config, handle).await,
        }
    }
}
