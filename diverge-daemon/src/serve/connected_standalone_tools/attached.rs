//! What a connected tool is attached to.

use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::store::{self, ConnectedToolId, agents, connected_standalone_tools};

/// The agents the tool is attached to directly, as keys, in the order
/// it was attached to them.
pub async fn agents_of(conn: &mut PgConnection, tool: ConnectedToolId) -> Result<Vec<key::Agent>, store::Error> {
    let mut keys = Vec::new();
    for id in connected_standalone_tools::attachments::of_tool(conn, tool).await? {
        if let Some(agent) = agents::by_id(conn, id, false).await? {
            keys.push(agent.key());
        }
    }
    Ok(keys)
}

/// The agent templates the tool is attached to, by id, in the order
/// it was attached to them.
pub async fn templates_of(conn: &mut PgConnection, tool: ConnectedToolId) -> Result<Vec<String>, store::Error> {
    connected_standalone_tools::template_attachments::of_tool(conn, tool).await
}
