pub mod logs;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Audio logs
    Logs { #[command(subcommand)] command: logs::Commands },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Logs { command } => command.handle(cli_config, handle).await,
        }
    }
}
