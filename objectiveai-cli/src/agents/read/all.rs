//! `agents read all` — return every queue item for the given
//! spawned agent id(s), regardless of watermark. Still advances
//! the per-(caller, spawned) watermark to the highest index, so a
//! subsequent `read pending` returns empty until new rows land.
//!
//! Companion to `read pending` (the watermark-respecting drain).
//! Emits one [`AgentItems`] notification per positional arg, in
//! order — same envelope as `read pending`. The caller id is taken
//! from `cli_config.agent_instance_hierarchy` (env-supplied `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY`,
//! defaulting to `"cli"` in `main.rs`).
//!
//! Positional args are **sub-ids**, not full composite ids — the
//! caller prefix is glued on internally.

use clap::Args;
use objectiveai_sdk::cli::output::{AgentItems, Handle, Notification, Output};

#[derive(Args)]
pub struct CommandArgs {
    /// Sub-ids (lineage-relative) of the spawned agents to read.
    /// The caller prefix is prepended internally — so for a caller
    /// of `cli` and a spawned `cli/foo-123`, pass `foo-123`. Repeat
    /// the positional argument for multiple agents; the command
    /// emits one `AgentItems` notification per arg, e.g.
    /// `agents read all foo-123 bar-456` emits two notifications.
    #[arg(required = true)]
    pub agent_instance_hierarchys: Vec<String>,
}

pub async fn handle(
    args: CommandArgs,
    cli_config: &crate::Config,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    let client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        None::<String>,
        None::<String>,
    );
    let caller = &cli_config.agent_instance_hierarchy;

    for sub in &args.agent_instance_hierarchys {
        let spawned = format!("{caller}/{sub}");
        let items = client.read_all_from_queue(caller, &spawned).await?;

        Output::Notification(Notification {
            value: (AgentItems {
                agent_id: sub.clone(),
                items,
            })
            .into(),
        })
        .emit(handle)
        .await;
    }

    Ok(())
}
