use clap::Subcommand;
use objectiveai_sdk::cli::output::Handle;

use crate::instance::api::{BodySource, HttpArgs, PipeArgs};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Stream `/functions/executions`. The streaming chunks are
    /// emitted one-per-NDJSON-line on stdout. Per-agent named pipes
    /// appear under `${config_base_dir}/pipes/<agent_instance_hierarchy>` for as
    /// long as that agent is in flight; external processes can
    /// connect and write NDJSON `RichContent` lines to push
    /// notifications at the agent. Log files land under
    /// `${config_base_dir}/logs/functions/executions/<fexc-id>/`,
    /// and a `LogStreamReady` notification is emitted on stdout
    /// once the root log id is available.
    Create {
        #[command(flatten)]
        body: BodySource,
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
            Commands::Create { body } => super::create::handle(http, pipes, body, handle).await,
        }
    }
}
