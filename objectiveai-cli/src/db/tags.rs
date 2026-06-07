//! Client-side agent tags backed by the postgres `tags` table.
//!
//! Each row is in exactly one of two states (a CHECK constraint
//! enforces mutual exclusion at the schema level):
//!
//! - **BOUND** — `agent_instance_hierarchy` set, the other two NULL.
//!   The tag points at one specific hierarchy.
//! - **PENDING** — `parent_agent_instance_hierarchy + agent_full_id`
//!   set, `agent_instance_hierarchy` NULL. The tag is waiting for the
//!   next agent-completion that matches the (full_id, parent) pair to
//!   spawn under it; its first chunk auto-promotes the row to BOUND
//!   via [`upgrade`].
//!
//! Re-tagging uses `INSERT … ON CONFLICT (name) DO UPDATE SET …` (the
//! postgres analog of sqlite's `INSERT OR REPLACE`), so the prior
//! binding is silently displaced.

use sqlx::Row as _;

use super::{Error, Pool};

/// Three-state result for a tag-name lookup. `Bound` is the only state
/// callers can act on directly; `Pending` carries enough information
/// for the read handlers to surface a "tag exists but hasn't been
/// spawned yet" diagnostic; `Absent` means the tag was never registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupState {
    Bound {
        agent_instance_hierarchy: String,
    },
    Pending {
        parent_agent_instance_hierarchy: String,
        agent_full_id: String,
    },
    Absent,
}

/// Parent scope of an `agent_instance_hierarchy`: the substring up to
/// (but not including) the last `/`. When the input has no `/`, the
/// parent is the empty string.
pub fn parent_of(agent_instance_hierarchy: &str) -> &str {
    match agent_instance_hierarchy.rfind('/') {
        Some(i) => &agent_instance_hierarchy[..i],
        None => "",
    }
}

/// Leaf segment of an `agent_instance_hierarchy`: everything after the
/// last `/`. When the input has no `/`, the leaf is the whole string.
pub fn leaf_of(agent_instance_hierarchy: &str) -> &str {
    match agent_instance_hierarchy.rfind('/') {
        Some(i) => &agent_instance_hierarchy[i + 1..],
        None => agent_instance_hierarchy,
    }
}

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Upsert `name` into PENDING state. Re-tags by displacing any prior
/// row (BOUND or PENDING) at the same `name`.
pub async fn upsert_pending(
    pool: &Pool,
    name: &str,
    agent_full_id: &str,
    parent_agent_instance_hierarchy: &str,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO tags \
         (name, agent_instance_hierarchy, parent_agent_instance_hierarchy, agent_full_id, updated_at) \
         VALUES ($1, NULL, $2, $3, $4) \
         ON CONFLICT (name) DO UPDATE SET \
             agent_instance_hierarchy        = EXCLUDED.agent_instance_hierarchy, \
             parent_agent_instance_hierarchy = EXCLUDED.parent_agent_instance_hierarchy, \
             agent_full_id                   = EXCLUDED.agent_full_id, \
             updated_at                      = EXCLUDED.updated_at",
    )
    .bind(name)
    .bind(parent_agent_instance_hierarchy)
    .bind(agent_full_id)
    .bind(now_seconds())
    .execute(&**pool)
    .await?;
    Ok(())
}

/// Upsert `name` into BOUND state. Re-tags by displacing any prior row
/// (BOUND or PENDING) at the same `name`.
pub async fn upsert_bound(
    pool: &Pool,
    name: &str,
    agent_instance_hierarchy: &str,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO tags \
         (name, agent_instance_hierarchy, parent_agent_instance_hierarchy, agent_full_id, updated_at) \
         VALUES ($1, $2, NULL, NULL, $3) \
         ON CONFLICT (name) DO UPDATE SET \
             agent_instance_hierarchy        = EXCLUDED.agent_instance_hierarchy, \
             parent_agent_instance_hierarchy = EXCLUDED.parent_agent_instance_hierarchy, \
             agent_full_id                   = EXCLUDED.agent_full_id, \
             updated_at                      = EXCLUDED.updated_at",
    )
    .bind(name)
    .bind(agent_instance_hierarchy)
    .bind(now_seconds())
    .execute(&**pool)
    .await?;
    Ok(())
}

/// All tags currently bound to the given hierarchy, newest-bound first.
/// PENDING rows never match.
pub async fn tags_for_hierarchy(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    let rows = sqlx::query(
        "SELECT name FROM tags \
         WHERE agent_instance_hierarchy = $1 \
         ORDER BY updated_at DESC",
    )
    .bind(agent_instance_hierarchy)
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row.try_get::<String, _>(0)?);
    }
    Ok(out)
}

/// Hierarchy bound to a given tag. `None` for PENDING or absent rows.
pub async fn hierarchy_for_tag(
    pool: &Pool,
    name: &str,
) -> Result<Option<String>, Error> {
    let row = sqlx::query(
        "SELECT agent_instance_hierarchy FROM tags WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(&**pool)
    .await?;
    let cell: Option<Option<String>> = row.map(|r| r.try_get(0)).transpose()?;
    Ok(cell.flatten())
}

/// Look up a tag and report its precise state. One SELECT returns all
/// three nullable columns; the table's CHECK constraint guarantees
/// every row matches exactly one of the BOUND or PENDING patterns, so
/// the row tuple decoding is exhaustive.
pub async fn lookup(pool: &Pool, name: &str) -> Result<LookupState, Error> {
    let row = sqlx::query(
        "SELECT agent_instance_hierarchy, \
                parent_agent_instance_hierarchy, \
                agent_full_id \
         FROM tags WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(&**pool)
    .await?;
    let Some(row) = row else {
        return Ok(LookupState::Absent);
    };
    let bound: Option<String> = row.try_get(0)?;
    let pending_parent: Option<String> = row.try_get(1)?;
    let pending_full_id: Option<String> = row.try_get(2)?;
    match (bound, pending_parent, pending_full_id) {
        (Some(h), None, None) => Ok(LookupState::Bound {
            agent_instance_hierarchy: h,
        }),
        (None, Some(p), Some(f)) => Ok(LookupState::Pending {
            parent_agent_instance_hierarchy: p,
            agent_full_id: f,
        }),
        other => Err(Error::InvalidData(format!(
            "tags row for {name:?} violates state invariant: {other:?}"
        ))),
    }
}

/// First-chunk notification: tell the tags table about an
/// agent-completion's identity. In one UPDATE, every PENDING row whose
/// `(agent_full_id, parent_agent_instance_hierarchy)` matches
/// `(agent_full_id, parent_of(agent_instance_hierarchy))` is flipped
/// to BOUND against the full `agent_instance_hierarchy`. Returns the
/// names of every promoted tag (typically 0 or 1).
pub async fn upgrade(
    pool: &Pool,
    agent_full_id: &str,
    agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    let parent = parent_of(agent_instance_hierarchy);
    let rows = sqlx::query(
        "UPDATE tags \
         SET agent_instance_hierarchy = $1, \
             parent_agent_instance_hierarchy = NULL, \
             agent_full_id = NULL, \
             updated_at = $2 \
         WHERE agent_full_id = $3 \
           AND parent_agent_instance_hierarchy = $4 \
         RETURNING name",
    )
    .bind(agent_instance_hierarchy)
    .bind(now_seconds())
    .bind(agent_full_id)
    .bind(parent)
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row.try_get::<String, _>(0)?);
    }
    Ok(out)
}
