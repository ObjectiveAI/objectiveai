//! Deferred-prompt storage for `agents queue {add, list}`.
//!
//! Co-located in `tags.sqlite` with the `tags` table so the
//! queue-list leaf can JOIN prompts ⨝ tags in a single SELECT (to
//! surface each tag-keyed row's BOUND / PENDING / ABSENT state).
//! The connection slot is owned by [`super::tags`]; this module
//! piggybacks on it.
//!
//! ## Schema
//!
//! One table, `prompts`. Each row targets either an
//! `agent_instance_hierarchy` OR an `agent_tag`, never both
//! (enforced by `CHECK`). Tags are stored verbatim — no resolution
//! at enqueue time; a future reader will resolve at dequeue time.
//!
//! Atomic dequeue via `DELETE … RETURNING …` is the planned future
//! shape; the `(target, id)` partial indexes are sized for it.

use objectiveai_sdk::cli::command::agents::queue::list::{
    LookupState, ResponseItem,
};
use rusqlite::Connection;

use super::super::{Client, Error};

fn now_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Insert one row. Exactly one of `agent_instance_hierarchy` or
/// `agent_tag` must be `Some` (the table's `CHECK` constraint will
/// reject malformed rows at the DB layer). Returns the auto-
/// incremented `id` of the new row via `INSERT ... RETURNING id`.
pub fn insert(
    client: &Client,
    agent_instance_hierarchy: Option<&str>,
    agent_tag: Option<&str>,
    prompt: &str,
) -> Result<i64, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(
        "INSERT INTO prompts (agent_instance_hierarchy, agent_tag, prompt, enqueued_at) \
         VALUES (?1, ?2, ?3, ?4) \
         RETURNING id",
    )?;
    let id = stmt.query_row(
        rusqlite::params![agent_instance_hierarchy, agent_tag, prompt, now_seconds()],
        |r| r.get::<_, i64>(0),
    )?;
    Ok(id)
}

/// Async wrapper around [`insert`].
pub async fn insert_async(
    client: Client,
    agent_instance_hierarchy: Option<String>,
    agent_tag: Option<String>,
    prompt: String,
) -> Result<i64, Error> {
    tokio::task::spawn_blocking(move || {
        insert(
            &client,
            agent_instance_hierarchy.as_deref(),
            agent_tag.as_deref(),
            &prompt,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// List all queued prompts visible under `parent`. Every row —
/// Direct or Tag — must resolve to a parent matching the filter:
///
/// * **Direct rows:** `agent_instance_hierarchy` is a direct child
///   of `parent` (the same `LIKE 'parent/%' AND no_further_slash`
///   pattern `list_direct_active_children` uses).
/// * **BOUND tag rows (LEFT JOIN finds the tag):** the tag's bound
///   `agent_instance_hierarchy` is a direct child of `parent` — same
///   predicate, applied against the joined `t.agent_instance_hierarchy`.
/// * **PENDING tag rows:** the tag's stored
///   `parent_agent_instance_hierarchy` equals `parent` exactly.
/// * **ABSENT tag rows (LEFT JOIN returns no match):** excluded —
///   no parent info available.
///
/// Returned items are ordered by `prompts.id` (FIFO of enqueue).
pub fn list(client: &Client, parent: &str) -> Result<Vec<ResponseItem>, Error> {
    let conn = super::tags::connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem prompts db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(
        "SELECT p.id, \
                p.agent_instance_hierarchy, \
                p.agent_tag, \
                t.agent_instance_hierarchy        AS tag_bound_hierarchy, \
                t.parent_agent_instance_hierarchy AS tag_pending_parent, \
                t.agent_full_id                   AS tag_pending_full_id \
         FROM prompts p \
         LEFT JOIN tags t ON p.agent_tag = t.name \
         WHERE \
                /* Direct row: agent_instance_hierarchy is a direct child of ?1 */ \
                ( \
                    p.agent_instance_hierarchy IS NOT NULL \
                    AND p.agent_instance_hierarchy LIKE (?1 || '/%') \
                    AND instr(substr(p.agent_instance_hierarchy, length(?1) + 2), '/') = 0 \
                ) \
             /* BOUND tag: tag's bound hierarchy is a direct child of ?1 */ \
             OR ( \
                    p.agent_tag IS NOT NULL \
                    AND t.agent_instance_hierarchy IS NOT NULL \
                    AND t.agent_instance_hierarchy LIKE (?1 || '/%') \
                    AND instr(substr(t.agent_instance_hierarchy, length(?1) + 2), '/') = 0 \
                ) \
             /* PENDING tag: tag's stored parent equals ?1 exactly */ \
             OR ( \
                    p.agent_tag IS NOT NULL \
                    AND t.parent_agent_instance_hierarchy = ?1 \
                ) \
         ORDER BY p.id",
    )?;
    let rows = stmt
        .query_map([parent], |r| {
            let id: i64 = r.get(0)?;
            let agent_instance_hierarchy: Option<String> = r.get(1)?;
            let agent_tag: Option<String> = r.get(2)?;
            let tag_bound_hierarchy: Option<String> = r.get(3)?;
            let tag_pending_parent: Option<String> = r.get(4)?;
            let tag_pending_full_id: Option<String> = r.get(5)?;
            Ok((
                id,
                agent_instance_hierarchy,
                agent_tag,
                tag_bound_hierarchy,
                tag_pending_parent,
                tag_pending_full_id,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = Vec::with_capacity(rows.len());
    for (
        id,
        agent_instance_hierarchy,
        agent_tag,
        tag_bound_hierarchy,
        tag_pending_parent,
        tag_pending_full_id,
    ) in rows
    {
        if let Some(h) = agent_instance_hierarchy {
            // Direct row — strip the `parent/` prefix to recover the
            // bare leaf the user originally passed to
            // `agents queue add`. Falling back to `leaf_of` covers
            // the empty-parent / mismatch edge cases.
            let agent_instance = h
                .strip_prefix(&format!("{parent}/"))
                .map(str::to_string)
                .unwrap_or_else(|| super::tags::leaf_of(&h).to_string());
            out.push(ResponseItem::AgentInstance { id, agent_instance });
        } else if let Some(tag) = agent_tag {
            // The SQL `WHERE` clause only emits tag rows whose join
            // resolves to BOUND (column 1 set) or PENDING (cols 2+3
            // set). Anything else is unreachable — silently skip
            // such a row rather than panic.
            let Some(state) = (match (tag_bound_hierarchy, tag_pending_parent, tag_pending_full_id)
            {
                (Some(agent_instance_hierarchy), None, None) => Some(LookupState::Bound {
                    agent_instance_hierarchy,
                }),
                (None, Some(parent_agent_instance_hierarchy), Some(agent_full_id)) => {
                    Some(LookupState::Pending {
                        parent_agent_instance_hierarchy,
                        agent_full_id,
                    })
                }
                _ => None,
            }) else {
                continue;
            };
            out.push(ResponseItem::Tag {
                id,
                agent_tag: tag,
                state,
            });
        } else {
            // CHECK constraint guarantees this is unreachable, but
            // we don't panic — silently skip a malformed row.
        }
    }
    Ok(out)
}

/// Async wrapper around [`list`].
pub async fn list_async(client: Client, parent: String) -> Result<Vec<ResponseItem>, Error> {
    tokio::task::spawn_blocking(move || list(&client, &parent))
        .await
        .map_err(spawn_blocking_join_err)?
}

fn spawn_blocking_join_err(e: tokio::task::JoinError) -> Error {
    Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
}

// Avoid the otherwise-unused `Connection` import warning since the
// only direct rusqlite use is via `super::tags::connection`.
#[allow(dead_code)]
type _Conn = Connection;

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_client() -> (Client, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let client = Client::new(
            Some(tmp.path().to_path_buf()),
            Some("test"),
            Some("test@test"),
        );
        (client, tmp)
    }

    #[test]
    fn insert_direct_row_returns_id_starting_at_one() {
        let (c, _tmp) = fresh_client();
        let id = insert(&c, Some("root/A/inst-1"), None, "[]").unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn insert_tag_row_returns_id() {
        let (c, _tmp) = fresh_client();
        let id = insert(&c, None, Some("foo"), "[]").unwrap();
        assert_eq!(id, 1);
    }

    #[test]
    fn insert_with_both_targets_violates_check() {
        let (c, _tmp) = fresh_client();
        let err = insert(&c, Some("root/A"), Some("foo"), "[]");
        assert!(err.is_err(), "CHECK constraint must reject both columns set");
    }

    #[test]
    fn insert_with_neither_target_violates_check() {
        let (c, _tmp) = fresh_client();
        let err = insert(&c, None, None, "[]");
        assert!(err.is_err(), "CHECK constraint must reject neither column set");
    }

    #[test]
    fn ids_increment_across_inserts() {
        let (c, _tmp) = fresh_client();
        let a = insert(&c, Some("root/A/h1"), None, "[]").unwrap();
        let b = insert(&c, None, Some("t"), "[]").unwrap();
        let c2 = insert(&c, Some("root/A/h2"), None, "[]").unwrap();
        assert!(a < b && b < c2);
    }
}
