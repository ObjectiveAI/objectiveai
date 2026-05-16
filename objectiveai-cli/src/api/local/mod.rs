pub mod config;
pub mod claude_agent_sdk;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Local API configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Claude Agent SDK enabled
    ClaudeAgentSdk {
        #[command(subcommand)]
        command: claude_agent_sdk::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config, handle).await,
            Commands::ClaudeAgentSdk { command } => command.handle(cli_config, handle).await,
        }
    }
}
