//! Laboratory IDs attached to an agent target, backed by the postgres
//! `laboratory_attachments` table.
//!
//! A row attaches one `laboratory_id` to EITHER an
//! `agent_instance_hierarchy` (AIH) OR a `tag` — never both (a CHECK
//! constraint enforces exclusivity). A given laboratory is attached at
//! most once per target (a partial unique index per target column).
//! `laboratory_id` is an opaque external identifier.

use super::{Error, Pool};

/// Which target column a row is keyed on.
pub enum Target {
    /// Keyed on `tag`.
    Tag(String),
    /// Keyed on `agent_instance_hierarchy`.
    Aih(String),
}

impl Target {
    /// `(tag, agent_instance_hierarchy)` bind values — exactly one is
    /// `Some`, the other `None` (bound as SQL NULL).
    fn columns(&self) -> (Option<&str>, Option<&str>) {
        match self {
            Target::Tag(tag) => (Some(tag.as_str()), None),
            Target::Aih(aih) => (None, Some(aih.as_str())),
        }
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Attach `laboratory_id` to `target`. Returns `true` if a row was
/// inserted, `false` if it was already attached. Caller holds the
/// agent lock, so the select-then-insert is race-free.
pub async fn attach(
    pool: &Pool,
    target: &Target,
    laboratory_id: &str,
) -> Result<bool, Error> {
    let (tag, aih) = target.columns();
    let existing = sqlx::query(
        "SELECT 1 FROM objectiveai.laboratory_attachments \
         WHERE tag IS NOT DISTINCT FROM $1 \
           AND agent_instance_hierarchy IS NOT DISTINCT FROM $2 \
           AND laboratory_id = $3",
    )
    .bind(tag)
    .bind(aih)
    .bind(laboratory_id)
    .fetch_optional(&**pool)
    .await?;
    if existing.is_some() {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO objectiveai.laboratory_attachments \
         (tag, agent_instance_hierarchy, laboratory_id, created_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(tag)
    .bind(aih)
    .bind(laboratory_id)
    .bind(now())
    .execute(&**pool)
    .await?;
    Ok(true)
}

/// Detach `laboratory_id` from `target`. Returns `true` if a row was
/// deleted, `false` if there was nothing to delete. Caller holds the
/// agent lock.
pub async fn detach(
    pool: &Pool,
    target: &Target,
    laboratory_id: &str,
) -> Result<bool, Error> {
    let (tag, aih) = target.columns();
    let result = sqlx::query(
        "DELETE FROM objectiveai.laboratory_attachments \
         WHERE tag IS NOT DISTINCT FROM $1 \
           AND agent_instance_hierarchy IS NOT DISTINCT FROM $2 \
           AND laboratory_id = $3",
    )
    .bind(tag)
    .bind(aih)
    .bind(laboratory_id)
    .execute(&**pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
