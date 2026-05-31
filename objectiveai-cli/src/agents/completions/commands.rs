use clap::Subcommand;

use super::{logs, continuations, messages};

#[derive(Subcommand)]
pub enum Commands {
    /// Agent completion logs
    Logs {
        #[command(subcommand)]
        command: logs::Commands,
    },
    /// Agent completion continuations
    Continuations {
        #[command(subcommand)]
        command: continuations::Commands,
    },
    /// Agent completion messages
    Messages {
        #[command(subcommand)]
        command: messages::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Logs { command } => command.handle(cli_config, handle).await,
            Commands::Continuations { command } => command.handle(cli_config, handle).await,
            Commands::Messages { command } => command.handle(cli_config, handle).await,
        }
    }
}
