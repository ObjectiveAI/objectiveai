pub mod config;

use clap::{Subcommand, ValueEnum};

#[derive(Clone, ValueEnum)]
pub enum Remote {
    Github,
    Filesystem,
}

impl From<Remote> for objectiveai_sdk::Remote {
    fn from(r: Remote) -> Self {
        match r {
            Remote::Github => objectiveai_sdk::Remote::Github,
            Remote::Filesystem => objectiveai_sdk::Remote::Filesystem,
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Remote configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config, handle).await,
        }
    }
}
