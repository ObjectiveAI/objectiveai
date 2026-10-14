//! How a tool on record is kept running, by its template.

use diverge_sdk::daemon::endpoints::standalone_tools::templates::Mode;
use sqlx::PgConnection;

use super::Tool;
use crate::store::{Error, tools_templates};

/// The mode the tool's template says, for a created tool: transient,
/// or a service. `None` for a connected tool, which is another
/// daemon's and runs by that daemon's template, and for a created
/// tool whose template is gone.
pub async fn mode(conn: &mut PgConnection, tool: &Tool) -> Result<Option<Mode>, Error> {
    let Some(template) = tool.template() else {
        return Ok(None);
    };
    Ok(tools_templates::by_id(conn, template, false).await?.map(|record| record.template.r#type.mode))
}
