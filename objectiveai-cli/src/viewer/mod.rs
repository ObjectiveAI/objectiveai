pub mod address;
pub mod config;
pub mod kill;
pub mod port;
pub mod secret;
pub mod send;
pub mod signature;
pub mod spawn;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Viewer configuration
    Config {
        #[command(subcommand)]
        command: config::Commands,
    },
    /// Local viewer bind address
    Address {
        #[command(subcommand)]
        command: address::Commands,
    },
    /// Local viewer bind port
    Port {
        #[command(subcommand)]
        command: port::Commands,
    },
    /// Viewer secret
    Secret {
        #[command(subcommand)]
        command: secret::Commands,
    },
    /// Viewer signature
    Signature {
        #[command(subcommand)]
        command: signature::Commands,
    },
    /// Generate a new secret/signature pair
    GenerateSecretSignaturePair,
    /// POST a JSON body to a viewer HTTP route and wait for the
    /// response.
    Send {
        /// URL path on the viewer (must start with `/`), e.g.
        /// `/agent/completions` or `/plugin/<repository>/<route>`.
        path: String,
        /// JSON body to POST. Validated as well-formed JSON before
        /// send.
        body: String,
    },
    /// Spawn the `objectiveai-viewer` Tauri shell in the background.
    /// Errors if it's already running.
    Spawn,
    /// Terminate every running `objectiveai-viewer` process.
    /// Idempotent — succeeds with count = 0 if none were running.
    Kill,
}

impl Commands {
    pub async fn handle(
        self,
        cli_config: &crate::Config,
        handle: &objectiveai_sdk::cli::output::Handle,
    ) -> Result<(), crate::error::Error> {
        match self {
            Commands::Config { command } => command.handle(cli_config, handle).await,
            Commands::Address { command } => command.handle(cli_config, handle).await,
            Commands::Port { command } => command.handle(cli_config, handle).await,
            Commands::Secret { command } => command.handle(cli_config, handle).await,
            Commands::Signature { command } => command.handle(cli_config, handle).await,
            Commands::GenerateSecretSignaturePair => {
                let pair =
                    crate::filesystem::config::generate_viewer_secret_signature_pair();
                crate::config::emit_value(&pair, handle).await;
                Ok(())
            }
            Commands::Send { path, body } => send::run(cli_config, handle, &path, &body).await,
            Commands::Spawn => spawn::handle(cli_config, handle).await,
            Commands::Kill => kill::handle(cli_config, handle).await,
        }
    }
}
