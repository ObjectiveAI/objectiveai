use clap::Subcommand;

use super::{create, logs, instructions};

#[derive(Subcommand)]
pub enum Commands {
    /// Create a recursive function invention
    Create {
        #[command(subcommand)]
        command: create::Commands,
    },
    /// Manage recursive-invention instructions
    Instructions {
        #[command(subcommand)]
        command: instructions::Commands,
    },
    /// Read recursive invention logs
    Logs {
        #[command(subcommand)]
        command: logs::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Create { command } => command.handle(cli_config, handle).await,
            Commands::Instructions { command } => command.handle(cli_config, handle).await,
            Commands::Logs { command } => command.handle(cli_config, handle).await,
        }
    }
}
