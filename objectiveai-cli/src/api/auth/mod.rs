pub mod keys;
pub mod credits;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Keys {
        #[command(subcommand)]
        command: keys::Commands,
    },
    Credits {
        #[command(subcommand)]
        command: credits::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Keys { command } => command.handle(cli_config, handle).await,
            Commands::Credits { command } => command.handle(cli_config, handle).await,
        }
    }
}
