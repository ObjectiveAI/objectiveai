//! Making a agent template, or finding it made.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::templates::Template;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::Error;

/// What a new agent template is made of: the template, under the id
/// the caller hashed it to, and who makes it.
#[derive(Debug, Clone)]
pub struct New {
    /// The id: the template's hash.
    pub id: String,
    /// The template, whole.
    pub template: Template,
    /// Who makes it — kept only when the row is new.
    pub creator: Creator,
}

/// What a create came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The template is made: a new row, or one deleted earlier and
    /// now live again under its first creator and created time.
    Created,
    /// A live template of the hash was there already; nothing changed.
    Exists,
}

/// Insert the template, or revive a deleted row of the same id with
/// its tags cleared; a live row is left as it is.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let revived = sqlx::query(
        "INSERT INTO diverge.agents_templates (id, template, creator) VALUES ($1, $2, $3) \
         ON CONFLICT (id) DO UPDATE SET deleted = false, tags = '{}' WHERE diverge.agents_templates.deleted \
         RETURNING id",
    )
    .bind(&new.id)
    .bind(Json(&new.template))
    .bind(Json(&new.creator))
    .fetch_optional(&mut *conn)
    .await?;
    Ok(if revived.is_some() { Created::Created } else { Created::Exists })
}
