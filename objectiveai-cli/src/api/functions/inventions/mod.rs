pub mod post;
pub mod recursive;
pub mod state;
pub mod prompts;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Post(post::Args),
    Recursive {
        #[command(subcommand)]
        command: recursive::Commands,
    },
    State {
        #[command(subcommand)]
        command: state::Commands,
    },
    Prompts {
        #[command(subcommand)]
        command: prompts::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Post(args) => post::handle(args, cli_config, handle).await,
            Commands::Recursive { command } => command.handle(cli_config, handle).await,
            Commands::State { command } => command.handle(cli_config, handle).await,
            Commands::Prompts { command } => command.handle(cli_config, handle).await,
        }
    }
}
