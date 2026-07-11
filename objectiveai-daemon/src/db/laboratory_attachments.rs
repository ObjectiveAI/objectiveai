//! Laboratory IDs attached to an agent target, backed by the postgres
//! `laboratory_attachments` table.
//!
//! A row attaches one `laboratory_id` to EITHER an
//! `agent_instance_hierarchy` (AIH) OR a `tag` — never both (a CHECK
//! constraint enforces exclusivity). A given laboratory is attached at
//! most once per target (a partial unique index per target column).
//! `laboratory_id` is an opaque external identifier.

use sqlx::Row as _;

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

/// Attach `laboratory_id` to `target`, recording who attached it.
/// Returns `true` if a row was inserted, `false` if it was already
/// attached. NO LOCK is held (attach works at any time) — the
/// per-target partial unique index arbitrates concurrent attaches:
/// a losing racer's unique violation reads as "already attached".
pub async fn attach(
    pool: &Pool,
    target: &Target,
    laboratory_id: &str,
    attached_by: &str,
) -> Result<bool, Error> {
    let (tag, aih) = target.columns();
    let result = sqlx::query(
        "INSERT INTO objectiveai.laboratory_attachments \
         (tag, agent_instance_hierarchy, laboratory_id, created_at, attached_by) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(tag)
    .bind(aih)
    .bind(laboratory_id)
    .bind(now())
    .bind(attached_by)
    .execute(&**pool)
    .await;
    match result {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
            // Unique violation on the per-target partial index —
            // already attached (possibly by a concurrent racer).
            Ok(false)
        }
        Err(e) => Err(e.into()),
    }
}

/// Detach `laboratory_id` from `target`. Returns `true` if a row was
/// deleted, `false` if there was nothing to delete. NO LOCK is held —
/// the DELETE is inherently race-safe.
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

/// One attachment row, as `agents instances get` surfaces it.
pub struct AttachmentRecord {
    pub laboratory_id: String,
    /// Unix seconds (`created_at`).
    pub attached_at: i64,
    /// The AIH that ran the attach. `None` on rows predating tracking.
    pub attached_by: Option<String>,
}

/// The agent's EFFECTIVE attachments — the union the next spawn pass
/// dials: rows keyed on the AIH itself plus rows keyed on any of its
/// bound tags. Oldest-attached first; a laboratory attached through
/// multiple routes appears once (earliest attachment wins).
pub async fn effective_for_aih(
    pool: &Pool,
    agent_instance_hierarchy: &str,
    tags: &[String],
) -> Result<Vec<AttachmentRecord>, Error> {
    let rows = sqlx::query(
        "SELECT laboratory_id, created_at, attached_by \
         FROM objectiveai.laboratory_attachments \
         WHERE agent_instance_hierarchy = $1 OR tag = ANY($2) \
         ORDER BY created_at, id",
    )
    .bind(agent_instance_hierarchy)
    .bind(tags)
    .fetch_all(&**pool)
    .await?;
    let mut out: Vec<AttachmentRecord> = Vec::with_capacity(rows.len());
    for row in rows {
        let laboratory_id: String = row.try_get("laboratory_id")?;
        if out.iter().any(|r| r.laboratory_id == laboratory_id) {
            continue;
        }
        out.push(AttachmentRecord {
            laboratory_id,
            attached_at: row.try_get("created_at")?,
            attached_by: row.try_get("attached_by")?,
        });
    }
    Ok(out)
}

/// One attachment row seen from the LABORATORY side: the agent
/// target it is attached to (exactly one of `agent_instance_hierarchy`
/// / `tag`, per the table's CHECK), plus when and by whom.
pub struct TargetAttachment {
    pub agent_instance_hierarchy: Option<String>,
    pub tag: Option<String>,
    /// Unix seconds (`created_at`).
    pub attached_at: i64,
    /// The AIH that ran the attach. `None` on rows predating tracking.
    pub attached_by: Option<String>,
}

/// Every attachment row targeting `laboratory_id`, oldest first — the
/// reverse direction of [`list`]/[`effective_for_aih`], for the
/// `/laboratories/{id}` endpoint's record.
pub async fn list_for_laboratory(
    pool: &Pool,
    laboratory_id: &str,
) -> Result<Vec<TargetAttachment>, Error> {
    let rows = sqlx::query(
        "SELECT agent_instance_hierarchy, tag, created_at, attached_by \
         FROM objectiveai.laboratory_attachments \
         WHERE laboratory_id = $1 \
         ORDER BY created_at, id",
    )
    .bind(laboratory_id)
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(TargetAttachment {
            agent_instance_hierarchy: row.try_get("agent_instance_hierarchy")?,
            tag: row.try_get("tag")?,
            attached_at: row.try_get("created_at")?,
            attached_by: row.try_get("attached_by")?,
        });
    }
    Ok(out)
}

/// All laboratory ids attached to `target`, oldest-attached first.
pub async fn list(pool: &Pool, target: &Target) -> Result<Vec<String>, Error> {
    let (tag, aih) = target.columns();
    let rows = sqlx::query(
        "SELECT laboratory_id FROM objectiveai.laboratory_attachments \
         WHERE tag IS NOT DISTINCT FROM $1 \
           AND agent_instance_hierarchy IS NOT DISTINCT FROM $2 \
         ORDER BY created_at",
    )
    .bind(tag)
    .bind(aih)
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row.try_get::<String, _>(0)?);
    }
    Ok(out)
}
