pub mod config;
pub mod logs;
pub mod remote;
pub mod recursive;
pub mod state;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Recursive function invention
    Recursive {
        #[command(subcommand)]
        command: recursive::Commands,
    },
    /// Inventions configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Read invention logs
    Logs {
        #[command(subcommand)]
        command: logs::Commands,
    },
    /// Inventions remote
    Remote {
        #[command(subcommand)]
        command: remote::Commands,
    },
    /// Invention state
    State {
        #[command(subcommand)]
        command: state::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<crate::Output, crate::error::Error> {
        match self {
            Commands::Recursive { command } => command.handle(cli_config).await,
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::Logs { command } => command.handle(cli_config).await,
            Commands::Remote { command } => command.handle(cli_config).await,
            Commands::State { command } => command.handle(cli_config).await,
        }
    }
}
