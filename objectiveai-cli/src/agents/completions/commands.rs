use clap::Subcommand;

use super::{create, logs, continuations, messages, instructions};

#[derive(Subcommand)]
pub enum Commands {
    /// Create an agent completion
    Create {
        #[command(subcommand)]
        command: create::Commands,
    },
    /// Manage agent-completion instructions
    Instructions {
        #[command(subcommand)]
        command: instructions::Commands,
    },
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
    pub async fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Create { command } => command.handle(cli_config).await,
            Commands::Instructions { command } => command.handle(cli_config),
            Commands::Logs { command } => command.handle(cli_config).await,
            Commands::Continuations { command } => command.handle(cli_config).await,
            Commands::Messages { command } => command.handle(cli_config).await,
        }
    }
}
