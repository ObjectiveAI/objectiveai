use clap::Subcommand;
use objectiveai_sdk::cli::output::Handle;

use crate::instance::api::{BodySource, HttpArgs, PipeArgs};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Stream a spawned agent completion. Per-chunk NDJSON on
    /// stdout, per-agent pipes under
    /// `${config_base_dir}/pipes/<agent_instance_hierarchy>`, coalesced log files
    /// under `${config_base_dir}/logs/agents/completions/<acc-id>/`,
    /// and a one-shot `LogStreamReady` notification once the root
    /// log id is available.
    Spawn {
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
            Commands::Spawn { body } => super::spawn::handle(http, pipes, body, handle).await,
        }
    }
}
