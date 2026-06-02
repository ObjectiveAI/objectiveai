//! `agents read pending` — drain every unread queue row for the
//! given spawned agent id(s) from this CLI's perspective. Emits
//! one [`AgentItems`] notification per positional arg, in order,
//! each carrying the sub-id plus that agent's drained items
//! (possibly empty).
//!
//! Wraps [`crate::filesystem::Client::read_new_from_queue`]:
//! atomically advances the per-(caller, spawned) watermark in
//! `messages_queue` and returns each unread row as a typed
//! `QueueItem`. The caller id is taken from `cli_config.agent_instance_hierarchy`
//! (env-supplied `OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY`, defaulting to `"cli"` in
//! `main.rs`).
//!
//! Positional args are **sub-ids**, not full composite ids — the
//! caller prefix is glued on internally. So if the caller is `cli`
//! and the spawned agent is `cli/foo-123`, the invocation is
//! `agents read pending foo-123` (matches the output shape of
//! `agents list active`, which also drops the caller prefix).

use clap::Args;
use objectiveai_sdk::cli::output::{AgentItems, Handle, Notification, Output};

#[derive(Args)]
pub struct CommandArgs {
    /// Sub-ids (lineage-relative) of the spawned agents to drain.
    /// The caller prefix is prepended internally — so for a caller
    /// of `cli` and a spawned `cli/foo-123`, pass `foo-123`. Repeat
    /// the positional argument for multiple agents; the command
    /// emits one `AgentItems` notification per arg, e.g.
    /// `agents read pending foo-123 bar-456` emits two
    /// notifications.
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
        let items = client.read_new_from_queue(caller, &spawned).await?;

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
