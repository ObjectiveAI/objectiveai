pub mod logs;
pub mod logprobs;
pub mod image;
pub mod audio;
pub mod video;
pub mod file;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Message logs
    Logs { #[command(subcommand)] command: logs::Commands },
    /// Message logprobs
    Logprobs { #[command(subcommand)] command: logprobs::Commands },
    /// Message images
    Image { #[command(subcommand)] command: image::Commands },
    /// Message audio
    Audio { #[command(subcommand)] command: audio::Commands },
    /// Message video
    Video { #[command(subcommand)] command: video::Commands },
    /// Message files
    File { #[command(subcommand)] command: file::Commands },
}

impl Commands {
    pub async fn handle(self, cli_config: &crate::Config) -> Result<(), crate::error::Error> {
        match self {
            Commands::Logs { command } => command.handle(cli_config).await,
            Commands::Logprobs { command } => command.handle(cli_config).await,
            Commands::Image { command } => command.handle(cli_config).await,
            Commands::Audio { command } => command.handle(cli_config).await,
            Commands::Video { command } => command.handle(cli_config).await,
            Commands::File { command } => command.handle(cli_config).await,
        }
    }
}
