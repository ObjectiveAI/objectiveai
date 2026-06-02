use clap::Subcommand;
use objectiveai_sdk::cli::output::Handle;

use crate::instance::api::{HttpArgs, PipeArgs};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Function executions
    Executions {
        #[command(subcommand)]
        command: super::executions::Commands,
    },
    /// Function inventions
    Inventions {
        #[command(subcommand)]
        command: super::inventions::Commands,
    },
}

impl Commands {
    pub async fn handle(
        self,
        http: &HttpArgs,
        pipes: &PipeArgs,
        handle: &Handle,
    ) -> Result<(), String> {
        match self {
            Commands::Executions { command } => command.handle(http, pipes, handle).await,
            Commands::Inventions { command } => command.handle(http, pipes, handle).await,
        }
    }
}
