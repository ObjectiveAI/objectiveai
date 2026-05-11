pub mod logs;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Continuation logs
    Logs { #[command(subcommand)] command: logs::Commands },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Logs { command } => command.handle(cli_config).await,
        }
    }
}
