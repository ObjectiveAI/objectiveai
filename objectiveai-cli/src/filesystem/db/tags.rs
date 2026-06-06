//! Client-side agent tags stored in a dedicated `tags.sqlite` file
//! under `<base_dir>/`. Separate from the main `db.sqlite` so the
//! tag lifecycle is isolated from message-log churn.
//!
//! ## Schema
//!
//! One table, `tags`. Each row is in exactly one of two states (a
//! `CHECK` constraint enforces mutual exclusion):
//!
//! - **BOUND** — `agent_instance_hierarchy` set, the other two NULL.
//!   The tag points at one specific hierarchy.
//! - **PENDING** — `parent_agent_instance_hierarchy + agent_full_id`
//!   set, `agent_instance_hierarchy` NULL. The tag is waiting for the
//!   next agent-completion that matches the (full_id, parent) pair to
//!   spawn under it, at which point its first chunk auto-promotes the
//!   row to BOUND.
//!
//! Re-tagging always uses `INSERT OR REPLACE` on the `name` primary
//! key, so the prior binding (or pending row) is silently displaced.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use super::super::{Client, Error};

/// Returns the shared SQLite connection for this filesystem client's
/// dedicated `tags.sqlite`, opening (and initialising) it if necessary.
/// Same lazy-init / no-poison semantics as
/// [`super::connection::connection`].
pub fn connection(client: &Client) -> Result<Arc<Mutex<Connection>>, Error> {
    let mut guard = client
        .tags_db_conn_slot()
        .lock()
        .expect("filesystem tags db mutex poisoned");
    if let Some(conn) = guard.as_ref() {
        return Ok(conn.clone());
    }
    let db_path = client.tags_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    // FK cascade for the queue content-row tables. The existing
    // `tags` table has no FKs, so this is a no-op for that surface.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    init_tables(&conn)?;
    let arc = Arc::new(Mutex::new(conn));
    *guard = Some(arc.clone());
    Ok(arc)
}

/// Create every table this DB hosts. `tags.sqlite` started as a
/// single-table file but now also holds `prompts` (the deferred-
/// message queue from `agents queue add`) so the queue-list leaf
/// can JOIN prompts ⨝ tags in a single SELECT.
fn init_tables(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tags (\
            name                            TEXT PRIMARY KEY NOT NULL, \
            agent_instance_hierarchy        TEXT, \
            parent_agent_instance_hierarchy TEXT, \
            agent_full_id                   TEXT, \
            updated_at                      INTEGER NOT NULL, \
            CHECK ( \
                (agent_instance_hierarchy IS NOT NULL \
                 AND parent_agent_instance_hierarchy IS NULL \
                 AND agent_full_id IS NULL) \
                OR \
                (agent_instance_hierarchy IS NULL \
                 AND parent_agent_instance_hierarchy IS NOT NULL \
                 AND agent_full_id IS NOT NULL) \
            ) \
        );\
        CREATE INDEX IF NOT EXISTS tags_hierarchy_idx \
            ON tags(agent_instance_hierarchy);\
        CREATE INDEX IF NOT EXISTS tags_pending_match_idx \
            ON tags(agent_full_id, parent_agent_instance_hierarchy);\
        CREATE TABLE IF NOT EXISTS prompts (\
            id                       INTEGER PRIMARY KEY AUTOINCREMENT, \
            agent_instance_hierarchy TEXT, \
            agent_tag                TEXT, \
            prompt                   TEXT NOT NULL, \
            enqueued_at              INTEGER NOT NULL, \
            CHECK ( \
                (agent_instance_hierarchy IS NOT NULL AND agent_tag IS NULL) \
                OR \
                (agent_instance_hierarchy IS NULL AND agent_tag IS NOT NULL) \
            ) \
        );\
        CREATE INDEX IF NOT EXISTS prompts_hierarchy_idx \
            ON prompts(agent_instance_hierarchy, id) \
            WHERE agent_instance_hierarchy IS NOT NULL;\
        CREATE INDEX IF NOT EXISTS prompts_tag_idx \
            ON prompts(agent_tag, id) \
            WHERE agent_tag IS NOT NULL;\
        CREATE TABLE IF NOT EXISTS prompt_contents (\
            id        INTEGER PRIMARY KEY AUTOINCREMENT, \
            prompt_id INTEGER NOT NULL, \
            kind      TEXT NOT NULL \
                CHECK (kind IN ('text','image','audio','video','file')), \
            FOREIGN KEY (prompt_id) REFERENCES prompts(id) ON DELETE CASCADE\
        );\
        CREATE INDEX IF NOT EXISTS prompt_contents_prompt_idx \
            ON prompt_contents(prompt_id);\
        CREATE TABLE IF NOT EXISTS prompt_texts (\
            id   INTEGER PRIMARY KEY, \
            text TEXT NOT NULL, \
            FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE\
        );\
        CREATE TABLE IF NOT EXISTS prompt_images (\
            id     INTEGER PRIMARY KEY, \
            url    TEXT NOT NULL, \
            detail TEXT, \
            FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE\
        );\
        CREATE TABLE IF NOT EXISTS prompt_audios (\
            id     INTEGER PRIMARY KEY, \
            data   TEXT NOT NULL, \
            format TEXT NOT NULL, \
            FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE\
        );\
        CREATE TABLE IF NOT EXISTS prompt_videos (\
            id  INTEGER PRIMARY KEY, \
            url TEXT NOT NULL, \
            FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE\
        );\
        CREATE TABLE IF NOT EXISTS prompt_files (\
            id        INTEGER PRIMARY KEY, \
            file_data TEXT, \
            file_id   TEXT, \
            filename  TEXT, \
            file_url  TEXT, \
            FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE\
        );\
        ",
    )?;
    Ok(())
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
pub fn upsert_pending(
    client: &Client,
    name: &str,
    agent_full_id: &str,
    parent_agent_instance_hierarchy: &str,
) -> Result<(), Error> {
    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tags db connection mutex poisoned");
    conn.execute(
        "INSERT OR REPLACE INTO tags \
         (name, agent_instance_hierarchy, parent_agent_instance_hierarchy, agent_full_id, updated_at) \
         VALUES (?1, NULL, ?2, ?3, ?4)",
        rusqlite::params![
            name,
            parent_agent_instance_hierarchy,
            agent_full_id,
            now_seconds(),
        ],
    )?;
    Ok(())
}

/// Upsert `name` into BOUND state. Re-tags by displacing any prior
/// row at the same `name`.
pub fn upsert_bound(
    client: &Client,
    name: &str,
    agent_instance_hierarchy: &str,
) -> Result<(), Error> {
    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tags db connection mutex poisoned");
    conn.execute(
        "INSERT OR REPLACE INTO tags \
         (name, agent_instance_hierarchy, parent_agent_instance_hierarchy, agent_full_id, updated_at) \
         VALUES (?1, ?2, NULL, NULL, ?3)",
        rusqlite::params![name, agent_instance_hierarchy, now_seconds()],
    )?;
    Ok(())
}

/// All tags currently bound to the given hierarchy, newest-bound
/// first. A hierarchy can carry multiple tag names (the schema's PK
/// is the `name`, not the hierarchy) — every BOUND row whose
/// `agent_instance_hierarchy` matches is returned. PENDING rows
/// never match.
pub fn tags_for_hierarchy(
    client: &Client,
    agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tags db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(
        "SELECT name FROM tags \
         WHERE agent_instance_hierarchy = ?1 \
         ORDER BY updated_at DESC",
    )?;
    let rows = stmt
        .query_map([agent_instance_hierarchy], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Hierarchy bound to a given tag. Returns `None` when the row is
/// absent or in PENDING state (no binding yet).
pub fn hierarchy_for_tag(
    client: &Client,
    name: &str,
) -> Result<Option<String>, Error> {
    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tags db connection mutex poisoned");
    use rusqlite::OptionalExtension as _;
    let mut stmt = conn.prepare_cached(
        "SELECT agent_instance_hierarchy FROM tags WHERE name = ?1",
    )?;
    let row: Option<Option<String>> = stmt
        .query_row([name], |r| r.get::<_, Option<String>>(0))
        .optional()?;
    Ok(row.flatten())
}

/// Three-state result for a tag-name lookup. `Bound` is the only
/// state callers can act on directly; `Pending` carries enough
/// information for the read handlers to surface a "tag exists but
/// hasn't been spawned yet" diagnostic; `Absent` means the tag was
/// never registered.
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

/// Look up a tag and report its precise state. One SELECT returns all
/// three nullable columns; the table's CHECK constraint guarantees
/// every row matches exactly one of the BOUND or PENDING patterns,
/// so the row tuple decoding is exhaustive.
pub fn lookup(client: &Client, name: &str) -> Result<LookupState, Error> {
    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tags db connection mutex poisoned");
    use rusqlite::OptionalExtension as _;
    let mut stmt = conn.prepare_cached(
        "SELECT agent_instance_hierarchy, \
                parent_agent_instance_hierarchy, \
                agent_full_id \
         FROM tags WHERE name = ?1",
    )?;
    let row: Option<(Option<String>, Option<String>, Option<String>)> = stmt
        .query_row([name], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .optional()?;
    match row {
        Some((Some(h), None, None)) => Ok(LookupState::Bound {
            agent_instance_hierarchy: h,
        }),
        Some((None, Some(p), Some(f))) => Ok(LookupState::Pending {
            parent_agent_instance_hierarchy: p,
            agent_full_id: f,
        }),
        Some(other) => Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "tags row for {name:?} violates state invariant: {other:?}"
            ),
        ))),
        None => Ok(LookupState::Absent),
    }
}

/// Parent scope of an `agent_instance_hierarchy`: the substring up
/// to (but not including) the last `/`. When the input has no `/`,
/// the parent is the empty string — the hierarchy is at the root.
pub(crate) fn parent_of(agent_instance_hierarchy: &str) -> &str {
    match agent_instance_hierarchy.rfind('/') {
        Some(i) => &agent_instance_hierarchy[..i],
        None => "",
    }
}

/// Leaf segment of an `agent_instance_hierarchy`: everything after
/// the last `/`. When the input has no `/`, the leaf is the whole
/// string.
pub(crate) fn leaf_of(agent_instance_hierarchy: &str) -> &str {
    match agent_instance_hierarchy.rfind('/') {
        Some(i) => &agent_instance_hierarchy[i + 1..],
        None => agent_instance_hierarchy,
    }
}

/// First-chunk notification: tell the tags table about an
/// agent-completion's identity. In one UPDATE, every PENDING row
/// whose `(agent_full_id, parent_agent_instance_hierarchy)` matches
/// `(agent_full_id, parent_of(agent_instance_hierarchy))` is flipped
/// to BOUND against the full `agent_instance_hierarchy`. Rows in the
/// BOUND state never match (the CHECK constraint guarantees
/// `agent_full_id IS NULL` on BOUND rows). Returns the names of every
/// promoted tag (typically 0 or 1). The streaming layer calls this
/// unconditionally on every agent-completion first chunk; non-matches
/// are silent no-ops.
pub fn upgrade(
    client: &Client,
    agent_full_id: &str,
    agent_instance_hierarchy: &str,
) -> Result<Vec<String>, Error> {
    let parent = parent_of(agent_instance_hierarchy);
    let conn = connection(client)?;
    let conn = conn
        .lock()
        .expect("filesystem tags db connection mutex poisoned");
    let mut stmt = conn.prepare_cached(
        "UPDATE tags \
         SET agent_instance_hierarchy = ?1, \
             parent_agent_instance_hierarchy = NULL, \
             agent_full_id = NULL, \
             updated_at = ?2 \
         WHERE agent_full_id = ?3 \
           AND parent_agent_instance_hierarchy = ?4 \
         RETURNING name",
    )?;
    let rows = stmt
        .query_map(
            rusqlite::params![
                agent_instance_hierarchy,
                now_seconds(),
                agent_full_id,
                parent,
            ],
            |r| r.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Async wrapper.
pub async fn upsert_pending_async(
    client: Client,
    name: String,
    agent_full_id: String,
    parent_agent_instance_hierarchy: String,
) -> Result<(), Error> {
    tokio::task::spawn_blocking(move || {
        upsert_pending(
            &client,
            &name,
            &agent_full_id,
            &parent_agent_instance_hierarchy,
        )
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// Async wrapper.
pub async fn upsert_bound_async(
    client: Client,
    name: String,
    agent_instance_hierarchy: String,
) -> Result<(), Error> {
    tokio::task::spawn_blocking(move || {
        upsert_bound(&client, &name, &agent_instance_hierarchy)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// Async wrapper.
pub async fn tags_for_hierarchy_async(
    client: Client,
    agent_instance_hierarchy: String,
) -> Result<Vec<String>, Error> {
    tokio::task::spawn_blocking(move || {
        tags_for_hierarchy(&client, &agent_instance_hierarchy)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

/// Async wrapper.
pub async fn hierarchy_for_tag_async(
    client: Client,
    name: String,
) -> Result<Option<String>, Error> {
    tokio::task::spawn_blocking(move || hierarchy_for_tag(&client, &name))
        .await
        .map_err(spawn_blocking_join_err)?
}

/// Async wrapper for [`lookup`].
pub async fn lookup_async(
    client: Client,
    name: String,
) -> Result<LookupState, Error> {
    tokio::task::spawn_blocking(move || lookup(&client, &name))
        .await
        .map_err(spawn_blocking_join_err)?
}

/// Async wrapper.
pub async fn upgrade_async(
    client: Client,
    agent_full_id: String,
    agent_instance_hierarchy: String,
) -> Result<Vec<String>, Error> {
    tokio::task::spawn_blocking(move || {
        upgrade(&client, &agent_full_id, &agent_instance_hierarchy)
    })
    .await
    .map_err(spawn_blocking_join_err)?
}

fn spawn_blocking_join_err(e: tokio::task::JoinError) -> Error {
    Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
}

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
    fn pending_then_upgrade_binds_and_clears_match_fields() {
        let (c, _tmp) = fresh_client();
        upsert_pending(&c, "foo", "F", "root/A").unwrap();
        let promoted = upgrade(&c, "F", "root/A/inst-1").unwrap();
        assert_eq!(promoted, vec!["foo".to_string()]);
        assert_eq!(
            hierarchy_for_tag(&c, "foo").unwrap(),
            Some("root/A/inst-1".to_string())
        );
        assert_eq!(
            tags_for_hierarchy(&c, "root/A/inst-1").unwrap(),
            vec!["foo".to_string()]
        );
    }

    #[test]
    fn upgrade_with_no_match_returns_empty() {
        let (c, _tmp) = fresh_client();
        upsert_pending(&c, "foo", "F", "root/A").unwrap();
        let promoted = upgrade(&c, "OTHER", "root/A/inst-1").unwrap();
        assert!(promoted.is_empty());
        // Original PENDING row should be untouched.
        assert_eq!(hierarchy_for_tag(&c, "foo").unwrap(), None);
    }

    #[test]
    fn upgrade_strips_last_segment_for_parent_match() {
        let (c, _tmp) = fresh_client();
        // Pending registered against parent `root/A`.
        upsert_pending(&c, "foo", "F", "root/A").unwrap();
        // Spawn fires with hierarchy `root/A/inst-1` — parent is `root/A`.
        let promoted = upgrade(&c, "F", "root/A/inst-1").unwrap();
        assert_eq!(promoted, vec!["foo".to_string()]);
        // Same agent_full_id under a different parent should NOT promote
        // a row registered under `root/A`.
        upsert_pending(&c, "bar", "F", "root/B").unwrap();
        let nope = upgrade(&c, "F", "root/X/inst-9").unwrap();
        assert!(nope.is_empty());
    }

    #[test]
    fn upgrade_at_root_uses_empty_parent() {
        let (c, _tmp) = fresh_client();
        upsert_pending(&c, "rootless", "F", "").unwrap();
        // Hierarchy with no `/` → parent = "".
        let promoted = upgrade(&c, "F", "lonely-root").unwrap();
        assert_eq!(promoted, vec!["rootless".to_string()]);
        assert_eq!(
            hierarchy_for_tag(&c, "rootless").unwrap(),
            Some("lonely-root".to_string())
        );
    }

    #[test]
    fn upsert_bound_twice_swaps_target() {
        let (c, _tmp) = fresh_client();
        upsert_bound(&c, "t", "root/A/h1").unwrap();
        assert_eq!(
            hierarchy_for_tag(&c, "t").unwrap(),
            Some("root/A/h1".to_string())
        );
        upsert_bound(&c, "t", "root/A/h2").unwrap();
        assert_eq!(
            hierarchy_for_tag(&c, "t").unwrap(),
            Some("root/A/h2".to_string())
        );
        // h1 no longer carries t.
        assert!(tags_for_hierarchy(&c, "root/A/h1").unwrap().is_empty());
    }

    #[test]
    fn upsert_bound_then_upsert_pending_flips_state() {
        let (c, _tmp) = fresh_client();
        upsert_bound(&c, "t", "root/A/h1").unwrap();
        upsert_pending(&c, "t", "F", "root/A").unwrap();
        // hierarchy_for_tag treats PENDING as no binding.
        assert_eq!(hierarchy_for_tag(&c, "t").unwrap(), None);
        // h1 is detached.
        assert!(tags_for_hierarchy(&c, "root/A/h1").unwrap().is_empty());
        // upgrade can now bind it.
        let promoted = upgrade(&c, "F", "root/A/h2").unwrap();
        assert_eq!(promoted, vec!["t".to_string()]);
        assert_eq!(
            hierarchy_for_tag(&c, "t").unwrap(),
            Some("root/A/h2".to_string())
        );
    }

    #[test]
    fn hierarchy_for_tag_returns_none_for_pending_or_absent() {
        let (c, _tmp) = fresh_client();
        assert_eq!(hierarchy_for_tag(&c, "missing").unwrap(), None);
        upsert_pending(&c, "p", "F", "root/A").unwrap();
        assert_eq!(hierarchy_for_tag(&c, "p").unwrap(), None);
    }

    #[test]
    fn tags_for_hierarchy_returns_all_matches_newest_first() {
        let (c, _tmp) = fresh_client();
        // Two distinct tag names bound to the same hierarchy.
        upsert_bound(&c, "older", "root/A/h1").unwrap();
        // Ensure the second insert has a strictly-later `updated_at`
        // so the newest-first ordering is deterministic.
        std::thread::sleep(std::time::Duration::from_secs(1));
        upsert_bound(&c, "newer", "root/A/h1").unwrap();
        let tags = tags_for_hierarchy(&c, "root/A/h1").unwrap();
        assert_eq!(tags, vec!["newer".to_string(), "older".to_string()]);
    }

    #[test]
    fn tags_for_hierarchy_returns_empty_for_unmatched() {
        let (c, _tmp) = fresh_client();
        upsert_bound(&c, "t", "root/A/h1").unwrap();
        assert!(tags_for_hierarchy(&c, "root/A/other").unwrap().is_empty());
    }
}
