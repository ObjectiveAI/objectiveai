//! `agents read id <id>` — resolve a queue Id (SQL row id in the
//! `files` table) to the underlying file's content. `.json` files
//! come back parsed as JSON; everything else is encoded as a
//! `data:` URL. Same `LogContent` wire shape every other `logs get`
//! family already emits.

use clap::Args;
use objectiveai_sdk::cli::output::Handle;

#[derive(Args)]
pub struct CommandArgs {
    /// SQL row id from a `files`-table entry — same integer you see
    /// in the refs inside `agents read pending` / `read all` output
    /// (`AgentItems.items[].…`).
    pub id: i64,
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
    let content = client.read_file_by_id(args.id).await?;
    crate::log_line::emit_log_content(content, handle).await;
    Ok(())
}
