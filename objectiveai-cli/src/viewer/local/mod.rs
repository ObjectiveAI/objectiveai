pub mod config;
pub mod secret;
pub mod signature;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Local viewer configuration
    Config { #[command(subcommand)] command: config::Commands },
    /// Viewer secret
    Secret { #[command(subcommand)] command: secret::Commands },
    /// Viewer signature
    Signature { #[command(subcommand)] command: signature::Commands },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config).await,
            Commands::Secret { command } => command.handle(cli_config).await,
            Commands::Signature { command } => command.handle(cli_config).await,
        }
    }
}
