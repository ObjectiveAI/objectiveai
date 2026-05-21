pub mod notify;
pub mod post;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Post(post::Args),
    Notify {
        #[command(subcommand)]
        command: notify::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Post(args) => post::handle(args, cli_config, handle).await,
            Commands::Notify { command } => command.handle(cli_config, handle).await,
        }
    }
}
