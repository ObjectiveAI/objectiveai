//! Holding a resource: a new record, a revived one, or a live one
//! told its latest description.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::resources::Kind;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::kind_text;
use crate::store::Error;

/// What an upload holds: the id the bytes hashed to, their kind and
/// size, what they are for, and who holds them.
#[derive(Debug, Clone)]
pub struct New {
    /// The id: the hash.
    pub id: String,
    /// Which kind.
    pub kind: Kind,
    /// What it is for.
    pub description: String,
    /// The size.
    pub bytes: u64,
    /// Who holds it — kept only when the row is new.
    pub creator: Creator,
}

/// What a hold came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    /// The resource is held anew: a new row, or one deleted earlier
    /// and live again under its first creator and created time.
    Held,
    /// The resource was held already; its description is now the
    /// request's, and nothing else changed.
    Exists,
}

/// Hold the resource. The row is locked for the transaction, so two
/// uploads of one content take turns at it.
pub async fn hold(conn: &mut PgConnection, new: &New) -> Result<Held, Error> {
    let existing = sqlx::query("SELECT deleted FROM diverge.resources WHERE id = $1 FOR UPDATE")
        .bind(&new.id)
        .fetch_optional(&mut *conn)
        .await?;
    match existing {
        None => {
            sqlx::query("INSERT INTO diverge.resources (id, kind, description, bytes, creator) VALUES ($1, $2, $3, $4, $5)")
                .bind(&new.id)
                .bind(kind_text(new.kind))
                .bind(&new.description)
                .bind(i64::try_from(new.bytes).unwrap_or(i64::MAX))
                .bind(Json(&new.creator))
                .execute(&mut *conn)
                .await?;
            Ok(Held::Held)
        }
        Some(row) => {
            let deleted: bool = row.try_get("deleted")?;
            sqlx::query("UPDATE diverge.resources SET description = $2, deleted = false, tags = CASE WHEN deleted THEN '{}' ELSE tags END WHERE id = $1")
                .bind(&new.id)
                .bind(&new.description)
                .execute(&mut *conn)
                .await?;
            Ok(if deleted { Held::Held } else { Held::Exists })
        }
    }
}
