pub mod config;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// `X-Title` header (kept as `x_title` because that is the literal
    /// HTTP header name)
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config, handle).await,
        }
    }
}
