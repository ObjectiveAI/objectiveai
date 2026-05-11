pub mod config;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Config { #[command(subcommand)] command: config::Commands },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self { Commands::Config { command } => command.handle(cli_config).await }
    }
}
