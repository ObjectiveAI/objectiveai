pub mod post;
pub mod votes;
pub mod cache;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Post(post::Args),
    Votes {
        #[command(subcommand)]
        command: votes::Commands,
    },
    Cache {
        #[command(subcommand)]
        command: cache::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Post(args) => post::handle(args, cli_config, handle).await,
            Commands::Votes { command } => command.handle(cli_config, handle).await,
            Commands::Cache { command } => command.handle(cli_config, handle).await,
        }
    }
}
