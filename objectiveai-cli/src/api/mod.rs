pub mod config;
pub mod mode;
pub mod remote;
pub mod local;
pub mod headers;
mod run;
pub mod detach;

pub use run::*;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// API configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// API mode (remote or local)
    Mode {
        #[command(subcommand)]
        command: mode::Commands,
    },
    /// Remote API configuration
    Remote {
        #[command(subcommand)]
        command: remote::Commands,
    },
    /// Local API configuration
    Local {
        #[command(subcommand)]
        command: local::Commands,
    },
    /// API headers configuration
    Headers {
        #[command(subcommand)]
        command: headers::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::Mode { command } => command.handle(cli_config).await,
            Commands::Remote { command } => command.handle(cli_config).await,
            Commands::Local { command } => command.handle(cli_config).await,
            Commands::Headers { command } => command.handle(cli_config).await,
        }
    }
}
