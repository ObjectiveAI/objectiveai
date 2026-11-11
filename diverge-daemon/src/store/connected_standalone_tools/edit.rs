//! Renaming a connected tool: the one thing an edit changes of one.

use sqlx::PgConnection;

use crate::store::{ConnectedToolId, Error, tool_names};

/// What a rename came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renamed {
    /// The tool is called as given.
    Renamed,
    /// The name is another standalone or connected tool's already;
    /// nothing changed.
    InUse,
}

/// Call the tool `name` — `None`, no name — claimed across both tool
/// families as [`tool_names`] states; `current` is the name it has now.
pub async fn rename(conn: &mut PgConnection, id: ConnectedToolId, current: Option<&str>, name: Option<&str>) -> Result<Renamed, Error> {
    if tool_names::claim(conn, name, current).await? {
        return Ok(Renamed::InUse);
    }
    let written = sqlx::query("UPDATE diverge.connected_standalone_tools SET name = $2 WHERE id = $1")
        .bind(id.0)
        .bind(name)
        .execute(&mut *conn)
        .await;
    match written {
        Ok(_) => Ok(Renamed::Renamed),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Renamed::InUse),
        Err(error) => Err(error.into()),
    }
}
