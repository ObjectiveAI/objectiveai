//! The agent templates whose agents ever attached a global service:
//! what holds the service's database scope up.
//!
//! A global service is nobody's in particular, so its scope cannot go
//! with an agent or an agent template as the other scopes do. It goes
//! with the last agent template whose agents ever declared it: a
//! row is written at every attachment, read at an agent template's
//! delete, and the templates left with no owner are the scopes to
//! sweep.

use sqlx::{PgConnection, Row as _};

use crate::store::Error;

/// The agent template's agents attached a global service of the
/// dependency tool template. One already noted stays as it was.
pub async fn add(conn: &mut PgConnection, template: &str, agent_template: &str) -> Result<(), Error> {
    sqlx::query("INSERT INTO diverge.service_owners (template, agent_template) VALUES ($1, $2) ON CONFLICT (template, agent_template) DO NOTHING")
        .bind(template)
        .bind(agent_template)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The agent template is deleted: taken off every global service it
/// owned a share of, and the dependency tool templates left with no
/// owner at all answered, each one a global scope to sweep.
pub async fn remove_template(conn: &mut PgConnection, agent_template: &str) -> Result<Vec<String>, Error> {
    let rows = sqlx::query("DELETE FROM diverge.service_owners WHERE agent_template = $1 RETURNING template")
        .bind(agent_template)
        .fetch_all(&mut *conn)
        .await?;
    let mut orphaned = Vec::new();
    for row in &rows {
        let template: String = row.try_get("template")?;
        let left: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM diverge.service_owners WHERE template = $1)")
            .bind(&template)
            .fetch_one(&mut *conn)
            .await?;
        if !left {
            orphaned.push(template);
        }
    }
    Ok(orphaned)
}
