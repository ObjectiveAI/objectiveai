pub mod completions;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Vector completions
    Completions { #[command(subcommand)] command: completions::Commands },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Completions { command } => command.handle(cli_config, handle).await,
        }
    }
}
