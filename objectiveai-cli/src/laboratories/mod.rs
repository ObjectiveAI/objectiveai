pub mod executions;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Laboratory executions
    Executions {
        #[command(subcommand)]
        command: executions::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Executions { command } => command.handle(cli_config).await,
        }
    }
}
