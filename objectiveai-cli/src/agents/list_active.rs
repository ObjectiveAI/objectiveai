//! `agents list-active` — list every direct-child agent of a
//! parent agent id along with the timestamp of its most recent
//! `assistant_response` row.
//!
//! Thin wrapper around
//! [`objectiveai_sdk::filesystem::Client::list_active`]. Direct
//! children only; deeper descendants (composite ids with more
//! than one segment past the parent) are excluded.

use clap::Args;
use objectiveai_sdk::cli::output::{ActiveAgent, Handle, Items, Notification, Output};

#[derive(Args)]
pub struct CommandArgs {
    /// Parent agent id whose direct children to enumerate. Defaults
    /// to the CLI's own `agent_id` (env `OBJECTIVEAI_AGENT_ID`,
    /// falling back to `"cli"` set in `main.rs`).
    pub parent_agent_id: Option<String>,
}

pub async fn handle(
    args: CommandArgs,
    cli_config: &crate::Config,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    let client = objectiveai_sdk::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        None::<String>,
        None::<String>,
    );
    let parent = args
        .parent_agent_id
        .as_deref()
        .unwrap_or(&cli_config.agent_id);

    let items = client.list_active(parent).await?;

    Output::Notification(Notification {
        agent_id: None,
        value: objectiveai_sdk::cli::output::NotificationValue::other(&(Items { items })),
    })
    .emit(handle)
    .await;
    Ok(())
}
