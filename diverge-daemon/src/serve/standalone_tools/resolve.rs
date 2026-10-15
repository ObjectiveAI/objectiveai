//! The tool a request names: a record, and whether the grants reach
//! it.

use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::{active, agents_of, templates_of};
use crate::daemon::Daemon;
use crate::judge::filter::standalone_tools::Facts;
use crate::judge::{self, Standing};
use crate::store::tools::Tool;
use crate::store::{self, tools};

/// The tool the reference names, if any: by name, by template and
/// index, or by daemon and tool, a record, locked for the transaction
/// when `lock`.
pub async fn resolve(conn: &mut PgConnection, reference: &reference::Tool, lock: bool) -> Result<Option<Tool>, store::Error> {
    tools::by_reference(conn, reference, lock).await
}

/// Whether the standing may do `action` to the record: whether it is
/// active now, what it is attached to — agents and agent templates —
/// and its template's mode, loaded here.
pub async fn reaches(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, action: Over, tool: &Tool) -> Result<bool, store::Error> {
    let agents = agents_of(conn, tool.id).await?;
    let templates = templates_of(conn, tool.id).await?;
    let mode = tools::mode(conn, tool).await?;
    Ok(judge::standalone_tools::over(standing, action, &Facts::record(tool, active(daemon, tool.id).await, &agents, &templates, mode)))
}
