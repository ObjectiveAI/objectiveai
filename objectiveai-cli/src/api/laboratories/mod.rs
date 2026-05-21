pub mod executions;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// `POST /laboratories/executions`
    Executions {
        #[command(subcommand)]
        command: executions::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Executions { command } => command.handle(cli_config, handle).await,
        }
    }
}
