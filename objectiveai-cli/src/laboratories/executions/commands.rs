use clap::Subcommand;

use super::{create, create_args, logs, instructions};

#[derive(Subcommand)]
pub enum Commands {
    /// Create a laboratory execution
    Create {
        #[command(flatten)]
        args: create_args::CreateArgs,
    },
    /// Manage laboratory-execution instructions
    Instructions {
        #[command(subcommand)]
        command: instructions::Commands,
    },
    /// Laboratory execution logs
    Logs {
        #[command(subcommand)]
        command: logs::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Create { args } => create::handle(args, cli_config, handle).await,
            Commands::Instructions { command } => command.handle(cli_config, handle).await,
            Commands::Logs { command } => command.handle(cli_config, handle).await,
        }
    }
}
