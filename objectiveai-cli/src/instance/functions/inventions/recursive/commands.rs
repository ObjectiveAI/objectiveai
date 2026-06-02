use clap::Subcommand;
use objectiveai_sdk::cli::output::Handle;

use crate::instance::api::{BodySource, HttpArgs, PipeArgs};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Stream `/functions/inventions/recursive`. Per-chunk NDJSON on
    /// stdout, per-agent pipes under `${config_base_dir}/pipes/<agent_instance_hierarchy>`,
    /// coalesced log files under
    /// `${config_base_dir}/logs/functions/inventions/recursive/<id>/`,
    /// and a one-shot `LogStreamReady` notification once the root
    /// log id is available.
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
