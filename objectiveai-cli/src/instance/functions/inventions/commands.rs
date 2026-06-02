use clap::Subcommand;
use objectiveai_sdk::cli::output::Handle;

use crate::instance::api::{HttpArgs, PipeArgs};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Recursive function inventions
    Recursive {
        #[command(subcommand)]
        command: super::recursive::Commands,
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
            Commands::Recursive { command } => command.handle(http, pipes, handle).await,
        }
    }
}
