pub mod list;
pub mod usage;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    List {
        #[command(subcommand)]
        command: list::Commands,
    },
    Usage {
        #[command(subcommand)]
        command: usage::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::List { command } => command.handle(cli_config, handle).await,
            Commands::Usage { command } => command.handle(cli_config, handle).await,
        }
    }
}
