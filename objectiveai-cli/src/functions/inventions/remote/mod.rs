pub mod config;

use clap::{Subcommand, ValueEnum};

#[derive(Clone, ValueEnum)]
pub enum Remote {
    Github,
    Filesystem,
}

impl From<Remote> for objectiveai::Remote {
    fn from(r: Remote) -> Self {
        match r {
            Remote::Github => objectiveai::Remote::Github,
            Remote::Filesystem => objectiveai::Remote::Filesystem,
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
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config).await,
        }
    }
}
