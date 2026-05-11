pub mod config;
pub mod objectiveai_address;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Remote API configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// ObjectiveAI address
    ObjectiveaiAddress {
        #[command(subcommand)]
        command: objectiveai_address::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::ObjectiveaiAddress { command } => command.handle(cli_config).await,
        }
    }
}
