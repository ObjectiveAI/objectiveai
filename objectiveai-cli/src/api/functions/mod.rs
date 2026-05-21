pub mod post;
pub mod list;
pub mod usage;
pub mod executions;
pub mod profiles;
pub mod inventions;

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
    Executions {
        #[command(subcommand)]
        command: executions::Commands,
    },
    Profiles {
        #[command(subcommand)]
        command: profiles::Commands,
    },
    Inventions {
        #[command(subcommand)]
        command: inventions::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Post(args) => post::handle(args, cli_config, handle).await,
            Commands::List { command } => command.handle(cli_config, handle).await,
            Commands::Usage { command } => command.handle(cli_config, handle).await,
            Commands::Executions { command } => command.handle(cli_config, handle).await,
            Commands::Profiles { command } => command.handle(cli_config, handle).await,
            Commands::Inventions { command } => command.handle(cli_config, handle).await,
        }
    }
}
