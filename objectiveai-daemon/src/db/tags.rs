//! Client-side agent tags backed by the postgres `tags` +
//! `tag_groups` tables.
//!
//! Each `tags` row is in exactly one of two states (a CHECK
//! constraint enforces mutual exclusion at the schema level):
//!
//! - **BOUND** — `agent_instance_hierarchy` set. The tag points at
//!   a live agent slot.
//! - **GROUPED** — `tag_group` set. The tag resolves through the
//!   referenced `tag_groups` row's `agent_spec` + parent. When the
//!   conduit's read-message-queue path fires its upgrade, every
//!   GROUPED tag sharing the spawn's group flips to BOUND atomically.
//!
//! The prior PENDING state is gone — group membership is now an
//! explicit join via `tag_group`, not an implicit
//! `(agent_full_id, parent_agent_instance_hierarchy)` match key.
//!
//! Re-tagging uses `INSERT … ON CONFLICT (name) DO UPDATE SET …`
//! (the postgres analog of sqlite's `INSERT OR REPLACE`), so the
//! prior binding is silently displaced.

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use sqlx::Row as _;

use super::{Error, Pool};

/// Three-state result for a tag-name lookup.
///
/// - `Bound` — the tag points to a live `agent_instance_hierarchy`.
/// - `Grouped` — the tag is a member of a `tag_groups` row carrying
///   an `AgentSpec` + parent. Spawning by this tag uses those two
///   fields directly; the upgrade flips all tags in this group to
///   `Bound` once any spawn picks them up.
/// - `Absent` — the tag was never registered.
#[derive(Debug, Clone, PartialEq)]
pub enum LookupState {
    Bound {
        agent_instance_hierarchy: String,
    },
    Grouped {
        tag_group_id: i64,
        agent_spec: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
        parent_agent_instance_hierarchy: String,
    },
    Absent,
}

/// Apply-target with the optional parent already substituted by
/// the cli's resolved hierarchy. The storage layer never deals
/// with the "default-to-ctx" rule — that's the CLI handler's job.
///
/// Mirrors the SDK's `ApplyTarget` but with all nullable defaults
/// pre-resolved. `AgentTag` forbids parent (the source tag's parent
/// is inherited via the group).
#[derive(Debug, Clone)]
pub enum ResolvedApplyTarget {
    /// `tag → AIH` where AIH = `{parent}/{agent_instance}`.
    AgentInstance {
        parent_agent_instance_hierarchy: String,
        agent_instance: String,
    },
    /// `tag → new tag_group`. Creates a fresh `tag_groups` row
    /// then a `tags` row pointing at it.
    Agent {
        parent_agent_instance_hierarchy: String,
        agent_spec: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    },
    /// Clone another tag's resolution. If source is BOUND, the
    /// new tag also binds to the same AIH. If source is GROUPED,
    /// the new tag joins the same `tag_group`.
    AgentTag {
        agent_tag: String,
    },
}

/// Parent scope of an `agent_instance_hierarchy`: the substring up
/// to (but not including) the last `/`. When the input has no `/`,
/// the parent is the empty string.
pub fn parent_of(agent_instance_hierarchy: &str) -> &str {
    match agent_instance_hierarchy.rfind('/') {
        Some(i) => &agent_instance_hierarchy[..i],
        None => "",
    }
}

/// Leaf segment of an `agent_instance_hierarchy`: everything after
/// the last `/`. When the input has no `/`, the leaf is the whole
/// string.
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

/// All tags currently bound to the given hierarchy, newest-bound
/// first. GROUPED rows never match (their hierarchy column is NULL).
pub async fn tags_for_hierarchy(
    pool: &Pool,
    agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    let rows = sqlx::query(
        "SELECT name FROM objectiveai.tags \
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

/// All tags belonging to the given tag_group, newest-updated first.
/// They upgrade together, so a spawn of any of them locks all of them.
pub async fn tags_for_group(pool: &Pool, tag_group: i64) -> Result<Vec<String>, Error> {
    let rows = sqlx::query(
        "SELECT name FROM objectiveai.tags \
         WHERE tag_group = $1 \
         ORDER BY updated_at DESC",
    )
    .bind(tag_group)
    .fetch_all(&**pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row.try_get::<String, _>(0)?);
    }
    Ok(out)
}

/// Hierarchy bound to a given tag. `None` for GROUPED rows and for
/// absent rows.
pub async fn hierarchy_for_tag(
    pool: &Pool,
    name: &str,
) -> Result<Option<String>, Error> {
    let row = sqlx::query(
        "SELECT agent_instance_hierarchy FROM objectiveai.tags WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(&**pool)
    .await?;
    let cell: Option<Option<String>> = row.map(|r| r.try_get(0)).transpose()?;
    Ok(cell.flatten())
}

/// Look up a tag and report its precise state. One LEFT JOIN
/// returns the BOUND column from `tags` plus the GROUPED columns
/// from `tag_groups`; the table's CHECK constraint guarantees
/// every row matches exactly one of the BOUND or GROUPED patterns,
/// so the row tuple decoding is exhaustive.
pub async fn lookup(pool: &Pool, name: &str) -> Result<LookupState, Error> {
    let row = sqlx::query(
        "SELECT t.agent_instance_hierarchy, \
                t.tag_group, \
                g.agent_spec, \
                g.parent_agent_instance_hierarchy \
         FROM objectiveai.tags t \
         LEFT JOIN objectiveai.tag_groups g ON g.id = t.tag_group \
         WHERE t.name = $1",
    )
    .bind(name)
    .fetch_optional(&**pool)
    .await?;
    let Some(row) = row else {
        return Ok(LookupState::Absent);
    };
    decode_lookup_row(&row, name)
}

/// Same decode used by `lookup` and by `apply`'s in-transaction
/// resolution of an `AgentTag` source. Single source of truth.
fn decode_lookup_row(
    row: &sqlx::postgres::PgRow,
    name: &str,
) -> Result<LookupState, Error> {
    let bound: Option<String> = row.try_get(0)?;
    let group_id: Option<i64> = row.try_get(1)?;
    let group_spec: Option<serde_json::Value> = row.try_get(2)?;
    let group_parent: Option<String> = row.try_get(3)?;
    match (bound, group_id, group_spec, group_parent) {
        (Some(h), None, None, None) => Ok(LookupState::Bound {
            agent_instance_hierarchy: h,
        }),
        (None, Some(id), Some(spec_value), Some(parent)) => {
            let agent_spec: InlineAgentBaseWithFallbacksOrRemoteCommitOptional =
                serde_json::from_value(spec_value)?;
            Ok(LookupState::Grouped {
                tag_group_id: id,
                agent_spec,
                parent_agent_instance_hierarchy: parent,
            })
        }
        other => Err(Error::InvalidData(format!(
            "tags row for {name:?} violates state invariant: {other:?}"
        ))),
    }
}

/// Apply a tag. Dispatches on the `ResolvedApplyTarget` variant
/// inside one transaction. `AgentTag` resolves the source tag
/// inline. Displaces any prior `tags` row for `name`. Returns the
/// freshly-applied `LookupState` so callers can surface it back
/// to the user without a second SELECT.
pub async fn apply(
    pool: &Pool,
    name: &str,
    target: ResolvedApplyTarget,
) -> Result<LookupState, Error> {
    let mut tx = pool.begin().await?;
    let now = now_seconds();
    let state = match target {
        ResolvedApplyTarget::AgentInstance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let hier = format!("{parent_agent_instance_hierarchy}/{agent_instance}");
            sqlx::query(
                "INSERT INTO objectiveai.tags (name, agent_instance_hierarchy, tag_group, updated_at) \
                 VALUES ($1, $2, NULL, $3) \
                 ON CONFLICT (name) DO UPDATE SET \
                     agent_instance_hierarchy = EXCLUDED.agent_instance_hierarchy, \
                     tag_group                = NULL, \
                     updated_at               = EXCLUDED.updated_at",
            )
            .bind(name)
            .bind(&hier)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            LookupState::Bound {
                agent_instance_hierarchy: hier,
            }
        }
        ResolvedApplyTarget::Agent {
            parent_agent_instance_hierarchy,
            agent_spec,
        } => {
            let spec_value = serde_json::to_value(&agent_spec)?;
            let group_id: i64 = sqlx::query_scalar(
                "INSERT INTO objectiveai.tag_groups (agent_spec, parent_agent_instance_hierarchy, created_at) \
                 VALUES ($1, $2, $3) RETURNING id",
            )
            .bind(&spec_value)
            .bind(&parent_agent_instance_hierarchy)
            .bind(now)
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO objectiveai.tags (name, agent_instance_hierarchy, tag_group, updated_at) \
                 VALUES ($1, NULL, $2, $3) \
                 ON CONFLICT (name) DO UPDATE SET \
                     agent_instance_hierarchy = NULL, \
                     tag_group                = EXCLUDED.tag_group, \
                     updated_at               = EXCLUDED.updated_at",
            )
            .bind(name)
            .bind(group_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            LookupState::Grouped {
                tag_group_id: group_id,
                agent_spec,
                parent_agent_instance_hierarchy,
            }
        }
        ResolvedApplyTarget::AgentTag { agent_tag } => {
            // Cycle detection: source tag can't be this name.
            // Longer cycles aren't possible since storage is
            // single-step (a tag points to either AIH or a group,
            // never another tag).
            if agent_tag == name {
                return Err(Error::InvalidData(format!(
                    "tag {name:?} cannot apply to itself"
                )));
            }
            let src = sqlx::query(
                "SELECT t.agent_instance_hierarchy, \
                        t.tag_group, \
                        g.agent_spec, \
                        g.parent_agent_instance_hierarchy \
                 FROM objectiveai.tags t \
                 LEFT JOIN objectiveai.tag_groups g ON g.id = t.tag_group \
                 WHERE t.name = $1",
            )
            .bind(&agent_tag)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                Error::InvalidData(format!(
                    "source tag {agent_tag:?} does not exist"
                ))
            })?;
            let src_state = decode_lookup_row(&src, &agent_tag)?;
            match src_state {
                LookupState::Bound {
                    agent_instance_hierarchy,
                } => {
                    sqlx::query(
                        "INSERT INTO objectiveai.tags (name, agent_instance_hierarchy, tag_group, updated_at) \
                         VALUES ($1, $2, NULL, $3) \
                         ON CONFLICT (name) DO UPDATE SET \
                             agent_instance_hierarchy = EXCLUDED.agent_instance_hierarchy, \
                             tag_group                = NULL, \
                             updated_at               = EXCLUDED.updated_at",
                    )
                    .bind(name)
                    .bind(&agent_instance_hierarchy)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
                    LookupState::Bound {
                        agent_instance_hierarchy,
                    }
                }
                LookupState::Grouped {
                    tag_group_id,
                    agent_spec,
                    parent_agent_instance_hierarchy,
                } => {
                    sqlx::query(
                        "INSERT INTO objectiveai.tags (name, agent_instance_hierarchy, tag_group, updated_at) \
                         VALUES ($1, NULL, $2, $3) \
                         ON CONFLICT (name) DO UPDATE SET \
                             agent_instance_hierarchy = NULL, \
                             tag_group                = EXCLUDED.tag_group, \
                             updated_at               = EXCLUDED.updated_at",
                    )
                    .bind(name)
                    .bind(tag_group_id)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
                    LookupState::Grouped {
                        tag_group_id,
                        agent_spec,
                        parent_agent_instance_hierarchy,
                    }
                }
                LookupState::Absent => unreachable!("loaded existing row above"),
            }
        }
    };
    tx.commit().await?;
    Ok(state)
}

/// What a removed tag was, plus the cleanup that rode the same
/// transaction. `Absent` = no such tag (the handler maps it to
/// [`Error::TagNotFound`](crate::error::Error)).
pub enum Removed {
    Absent,
    Bound {
        agent_instance_hierarchy: String,
        detached_laboratories: u64,
    },
    Grouped {
        tag_group_deleted: bool,
        detached_laboratories: u64,
    },
}

/// Delete a tag registration by name, whatever its shape, in ONE
/// transaction with its cleanup:
///
/// 1. `DELETE FROM tags … RETURNING` — the row's shape decides the
///    rest (the BOUND delete's row trigger fires `tags_changed` for
///    the vacated hierarchy; GROUPED deletes notify nothing, matching
///    their absence from instance records).
/// 2. Detach the tag's laboratory attachments (no FK exists;
///    attachments target existing tags, and orphans would silently
///    resurrect under a reused name). The attachments row trigger
///    notifies `laboratory_attachments_changed` per row.
/// 3. For a GROUPED tag, garbage-collect the now-possibly-empty
///    `tag_groups` row — with `SELECT … FOR UPDATE` FIRST: a bare
///    conditional DELETE races a concurrent `apply --agent-tag`
///    joining the group (the joiner holds only FOR KEY SHARE, so the
///    remover's NOT EXISTS evaluates against a pre-join snapshot and
///    the delete would CASCADE the freshly-committed sibling away).
///    FOR UPDATE conflicts with FOR KEY SHARE, so it blocks until any
///    in-flight join commits; the conditional DELETE's fresh READ
///    COMMITTED snapshot then sees the joiner and skips. A join that
///    starts after the lock instead fails its FK check — an error,
///    never a dangling reference. Two last-member removers racing can
///    both skip the GC and leave an orphan group: the same tolerated
///    outcome the GROUPED→BOUND upgrade path already produces.
pub async fn remove(pool: &Pool, name: &str) -> Result<Removed, Error> {
    let mut tx = pool.begin().await?;
    let Some(row) = sqlx::query(
        "DELETE FROM objectiveai.tags WHERE name = $1          RETURNING agent_instance_hierarchy, tag_group",
    )
    .bind(name)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(Removed::Absent);
    };
    let hierarchy: Option<String> = row.get("agent_instance_hierarchy");
    let tag_group: Option<i64> = row.get("tag_group");

    let detached_laboratories =
        sqlx::query("DELETE FROM objectiveai.laboratory_attachments WHERE tag = $1")
            .bind(name)
            .execute(&mut *tx)
            .await?
            .rows_affected();

    let removed = match (hierarchy, tag_group) {
        (Some(agent_instance_hierarchy), None) => Removed::Bound {
            agent_instance_hierarchy,
            detached_laboratories,
        },
        (None, Some(group)) => {
            sqlx::query("SELECT id FROM objectiveai.tag_groups WHERE id = $1 FOR UPDATE")
                .bind(group)
                .fetch_optional(&mut *tx)
                .await?;
            let tag_group_deleted = sqlx::query(
                "DELETE FROM objectiveai.tag_groups WHERE id = $1                  AND NOT EXISTS (SELECT 1 FROM objectiveai.tags WHERE tag_group = $1)",
            )
            .bind(group)
            .execute(&mut *tx)
            .await?
            .rows_affected()
                > 0;
            Removed::Grouped {
                tag_group_deleted,
                detached_laboratories,
            }
        }
        // The tags CHECK constraint makes any other shape impossible.
        _ => unreachable!("tags row is BOUND xor GROUPED by CHECK constraint"),
    };
    tx.commit().await?;
    Ok(removed)
}
