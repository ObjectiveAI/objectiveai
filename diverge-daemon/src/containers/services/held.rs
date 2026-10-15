//! A service's hold, recomputed from what holds it.

use std::time::Instant;

use crate::containers::ToolKey;
use crate::daemon::Daemon;
use sqlx::PgConnection;

use crate::store::{self, ToolId, dependency_tools, tools};

/// What holds the service changed: whether anything does read anew —
/// any agent attached, from the records, for either kind of tool, a
/// tool on record's to an agent or to an agent template; and for a
/// tool on record, any connect scope open on its run as well —
/// its state told, and the moment noted as the last change, which
/// its clock counts from. A service with no state — never kept, or
/// retired — is left as it is.
pub async fn held(daemon: &Daemon, key: ToolKey) {
    let Some(state) = daemon.live.service_state(key).await else {
        return;
    };
    let attached = match key {
        ToolKey::Record(id) => match daemon.store.acquire().await {
            Ok(mut conn) => attached_anywhere(&mut conn, id).await,
            Err(error) => Err(error),
        },
        ToolKey::Dependency(id) => match daemon.store.acquire().await {
            Ok(mut conn) => dependency_tools::attachments::attached(&mut conn, id).await.map(|agents| !agents.is_empty()),
            Err(error) => Err(error),
        },
    };
    // A store that cannot be read says nothing: the hold is left as it
    // was, rather than a service retired for want of an answer.
    let Ok(mut held) = attached else {
        return;
    };
    if !held {
        if let ToolKey::Record(_) = key {
            if let Some(run) = daemon.live.tool_run(key).await {
                held = !run.connections.lock().await.is_empty();
            }
        }
    }
    state.touched.send_replace(Instant::now());
    state.held.send_replace(held);
}

/// Whether the tool on record is attached to any agent or any agent
/// template, from the records.
async fn attached_anywhere(conn: &mut PgConnection, id: ToolId) -> Result<bool, store::Error> {
    if !tools::attachments::of_tool(conn, id).await?.is_empty() {
        return Ok(true);
    }
    Ok(!tools::template_attachments::of_tool(conn, id).await?.is_empty())
}
