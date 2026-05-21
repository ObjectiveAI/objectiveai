pub mod post;
pub mod list;
pub mod usage;
pub mod pairs;
pub mod compute;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Post(post::Args),
    List {
        #[command(subcommand)]
        command: list::Commands,
    },
    Usage {
        #[command(subcommand)]
        command: usage::Commands,
    },
    Pairs {
        #[command(subcommand)]
        command: pairs::Commands,
    },
    Compute {
        #[command(subcommand)]
        command: compute::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Post(args) => post::handle(args, cli_config, handle).await,
            Commands::List { command } => command.handle(cli_config, handle).await,
            Commands::Usage { command } => command.handle(cli_config, handle).await,
            Commands::Pairs { command } => command.handle(cli_config, handle).await,
            Commands::Compute { command } => command.handle(cli_config, handle).await,
        }
    }
}
