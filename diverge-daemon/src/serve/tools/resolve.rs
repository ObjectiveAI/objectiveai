//! The tool a request names: a record, and whether the grants reach
//! it.

use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::{active, agents_of};
use crate::daemon::Daemon;
use crate::judge::filter::tools::Facts;
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
/// active now and what it is attached to, loaded here.
pub async fn reaches(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, action: Over, tool: &Tool) -> Result<bool, store::Error> {
    let attached = agents_of(conn, tool.id).await?;
    Ok(judge::tools::over(standing, action, &Facts::record(tool, active(daemon, tool.id).await, &attached)))
}
