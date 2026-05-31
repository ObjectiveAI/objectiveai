use clap::Subcommand;

use super::{create, logs, retry_tokens};

#[derive(Subcommand)]
pub enum Commands {
    /// Create a function execution
    Create {
        #[command(subcommand)]
        command: create::Commands,
    },
    /// Function execution logs
    Logs {
        #[command(subcommand)]
        command: logs::Commands,
    },
    /// Retry tokens
    RetryTokens {
        #[command(subcommand)]
        command: retry_tokens::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Create { command } => command.handle(cli_config, handle).await,
            Commands::Logs { command } => command.handle(cli_config, handle).await,
            Commands::RetryTokens { command } => command.handle(cli_config, handle).await,
        }
    }
}
