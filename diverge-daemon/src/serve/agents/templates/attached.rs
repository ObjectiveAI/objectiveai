//! The tools attached to an agent template.

use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::store::{self, tools};

/// The tools attached to the template, as keys, in the order they
/// were attached.
pub async fn tools_of(conn: &mut PgConnection, template: &str) -> Result<Vec<key::Tool>, store::Error> {
    let mut keys = Vec::new();
    for id in tools::template_attachments::of_template(conn, template).await? {
        if let Some(tool) = tools::by_id(conn, id, false).await? {
            keys.push(tool.key());
        }
    }
    Ok(keys)
}
