//! Shared per-agent-id API for the `messages` table.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};

use objectiveai_sdk::agent::completions::message::RichContent;

use objectiveai_sdk::cli::command::agents::read::subscribe::RequestMessageKind;

use super::pending::PendingNotification;
use super::schema::{self, MessageRow, parse_message_kind};

/// Per-stream handle to the shared `messages` table API. Owns:
/// - the per-agent monotonic `next_index` counter,
/// - the per-agent "request row inserted" once-flag,
/// - the per-agent path-dedup set.
///
/// All db reads/writes flow through this type. `Clone` is cheap —
/// internal state is `Arc`-shared across clones, so the LogWriter,
/// the cli-stream writer task, and any future readers can hold their
/// own clone without contention beyond the per-agent mutex.
#[derive(Clone)]
pub struct Queue {
    inner: Arc<QueueInner>,
}

struct QueueInner {
    /// Shared SQLite connection (from [`super::connection::connection`]).
    conn: Arc<StdMutex<rusqlite::Connection>>,
    /// `${logs_dir}` — base for any files the queue writes
    /// (notification log files today).
    logs_dir: PathBuf,
    agents: StdMutex<HashMap<String, Arc<AgentEntry>>>,
}

struct AgentEntry {
    state: StdMutex<AgentMutableState>,
}

struct AgentMutableState {
    next_index: u64,
    request_inserted: bool,
    /// Per-(kind, path) dedup, scoped per agent (per `response_id`
    /// after lineage-stamping) via the enclosing `agents` HashMap.
    /// The full unique tuple is `(response_id, kind, path)` —
    /// `(kind, path)`, `(kind, response_id)`, and `(response_id, path)`
    /// can each collide alone:
    ///   - one writer drives multiple agent completions in the same
    ///     stream (e.g. inside a function execution), so two agents
    ///     can land identical `(kind, path)` rows — distinguished by
    ///     `response_id`.
    ///   - within one agent completion, the same chunk gets re-emitted
    ///     each `write` as the agg grows, so two consecutive calls can
    ///     produce identical `(kind, response_id)` rows — distinguished
    ///     by `path` (the bare message index).
    ///   - assistant and tool messages can land at distinct paths but
    ///     the reader dispatches by `kind`, so omitting `kind` would
    ///     let an assistant row mask a later tool row at the same
    ///     `(response_id, path)`.
    /// The HashMap key gives `response_id` for free; this set adds the
    /// remaining `(kind, path)`.
    inserted_paths: HashSet<(RequestMessageKind, String)>,
}

impl Queue {
    /// Build a Queue backed by the shared SQLite connection.
    /// `logs_dir` is still needed for the notification file write.
    pub fn new(
        conn: Arc<StdMutex<rusqlite::Connection>>,
        logs_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            inner: Arc::new(QueueInner {
                conn,
                logs_dir: logs_dir.into(),
                agents: StdMutex::new(HashMap::new()),
            }),
        }
    }

    /// Reserve and return the next monotonic db index for an agent.
    /// Seeds the agent's entry from `MAX(index) WHERE agent_instance_hierarchy = ?`
    /// + 1 on first use.
    pub async fn reserve_index(
        &self,
        agent_instance_hierarchy: &str,
    ) -> Result<u64, super::super::Error> {
        let entry = self.ensure_agent(agent_instance_hierarchy).await?;
        let mut state = entry.state.lock().expect("agent state mutex poisoned");
        let idx = state.next_index;
        state.next_index += 1;
        Ok(idx)
    }

    /// Insert one row at a caller-given index. `response_id` is the
    /// bare agent-completion chunk id; it's stored in its own column
    /// so the reader doesn't have to parse it back out of `agent_instance_hierarchy`.
    pub async fn insert(
        &self,
        agent_instance_hierarchy: &str,
        response_id: &str,
        kind: RequestMessageKind,
        path: String,
        timestamp: u64,
        index: u64,
    ) -> Result<(), super::super::Error> {
        self.ensure_agent(agent_instance_hierarchy).await?;
        schema::insert_async(
            Arc::clone(&self.inner.conn),
            agent_instance_hierarchy.to_string(),
            response_id.to_string(),
            kind,
            path,
            timestamp,
            index,
        )
        .await
    }

    /// Insert the per-stream request row at most once per agent.
    /// Reserves the next index under the same lock so concurrent
    /// callers can't race past the dedup check. Returns `true` if
    /// the row was inserted, `false` if a prior call already did it.
    pub async fn insert_request_once(
        &self,
        agent_instance_hierarchy: &str,
        response_id: &str,
        kind: RequestMessageKind,
        path: String,
        timestamp: u64,
    ) -> Result<bool, super::super::Error> {
        let entry = self.ensure_agent(agent_instance_hierarchy).await?;
        let index = {
            let mut state =
                entry.state.lock().expect("agent state mutex poisoned");
            if state.request_inserted {
                return Ok(false);
            }
            state.request_inserted = true;
            let idx = state.next_index;
            state.next_index += 1;
            idx
        };
        schema::insert_async(
            Arc::clone(&self.inner.conn),
            agent_instance_hierarchy.to_string(),
            response_id.to_string(),
            kind,
            path,
            timestamp,
            index,
        )
        .await?;
        Ok(true)
    }

    /// Register a `(kind, path)` pair for dedup under `agent_instance_hierarchy`.
    /// Returns `true` if newly inserted, `false` if already present
    /// (caller should skip the insert).
    ///
    /// The effective unique tuple is `(response_id, kind, path)` —
    /// `agent_instance_hierarchy`'s trailing segment is the response id (set by the
    /// chunk producer and lineage-stamped here), so the per-agent
    /// HashMap entry contributes `response_id` and this set adds
    /// `(kind, path)`. None of the pairwise subsets is unique on its
    /// own; see the `inserted_paths` field comment for the cases each
    /// missing axis would alias.
    pub async fn register_path(
        &self,
        agent_instance_hierarchy: &str,
        kind: RequestMessageKind,
        path: &str,
    ) -> Result<bool, super::super::Error> {
        let entry = self.ensure_agent(agent_instance_hierarchy).await?;
        let mut state = entry.state.lock().expect("agent state mutex poisoned");
        Ok(state.inserted_paths.insert((kind, path.to_string())))
    }

    /// Write a notification's content out as per-leaf files (text /
    /// media parts under
    /// `agents/completions/request/notifications/{text,image,...}/`)
    /// plus a parent `RichContentLog` envelope at
    /// `agents/completions/request/notifications/<response_id>_<idx>.json`,
    /// reserve the agent's next index, and return a
    /// [`PendingNotification`] the caller queues locally for a later
    /// [`Self::insert_notification`] call. Same extract-to-leaves
    /// pattern that messages use; the on-disk DB row's `path` column
    /// holds just `{idx}` (route + `response_id` are reconstructed
    /// from the kind + the new column).
    ///
    /// `response_id` is the agent completion the notification targets
    /// — the same value `AgentCompletionNotifyParams.response_id`
    /// carries on the wire. Stored explicitly; never re-derived from
    /// `agent_instance_hierarchy`.
    pub async fn write_notification(
        &self,
        agent_instance_hierarchy: &str,
        response_id: &str,
        content: &RichContent,
    ) -> Result<PendingNotification, super::super::Error> {
        let entry = self.ensure_agent(agent_instance_hierarchy).await?;
        let index = {
            let mut state =
                entry.state.lock().expect("agent state mutex poisoned");
            let idx = state.next_index;
            state.next_index += 1;
            idx
        };

        // 1. Extract the content into per-leaf files; their parent
        //    directory is `request/notifications/{text,image,...}/`.
        //    `response_id` keys every extracted leaf so it stays
        //    aligned with the envelope filename below.
        let (content_log, leaf_files) =
            crate::logs::agents::completions::message::rich_content::extract_media(
                content.clone(),
                "agents/completions/request/notifications",
                response_id,
                index,
            );

        // 2. Write the per-leaf files.
        for file in leaf_files {
            let full_path = self.inner.logs_dir.join(file.path());
            if let Some(parent) = full_path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    super::super::Error::Write(parent.to_path_buf(), e)
                })?;
            }
            tokio::fs::write(&full_path, file.content)
                .await
                .map_err(|e| super::super::Error::Write(full_path, e))?;
        }

        // 3. Write the parent envelope (a bare `RichContentLog`).
        let rel_path = format!(
            "agents/completions/request/notifications/{response_id}_{index}.json"
        );
        let full_path = self.inner.logs_dir.join(&rel_path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                super::super::Error::Write(parent.to_path_buf(), e)
            })?;
        }
        let bytes = serde_json::to_vec_pretty(&content_log)
            .map_err(super::super::Error::Serialize)?;
        tokio::fs::write(&full_path, bytes)
            .await
            .map_err(|e| super::super::Error::Write(full_path, e))?;

        Ok(PendingNotification {
            agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
            response_id: response_id.to_string(),
            index,
            // DB column stores just the bare index.
            path: format!("{index}"),
            timestamp: now_secs(),
        })
    }

    /// Insert a previously-reserved notification row at its already-
    /// reserved index.
    pub async fn insert_notification(
        &self,
        notification: PendingNotification,
    ) -> Result<(), super::super::Error> {
        self.insert(
            &notification.agent_instance_hierarchy,
            &notification.response_id,
            RequestMessageKind::AgentCompletionNotification,
            notification.path,
            notification.timestamp,
            notification.index,
        )
        .await
    }

    /// Read every message for `spawned_agent_instance_hierarchy` whose `index` is
    /// strictly greater than `caller_agent_instance_hierarchy`'s watermark in
    /// `messages_queue`, then upsert the watermark to the max
    /// returned index. Returns the matching `MessageRow`s in
    /// ascending index order.
    ///
    /// First-read semantics: when no `messages_queue` row exists for
    /// the pair, the watermark defaults to `0`. The query is strict
    /// `>`, so `index = 0` (typically the request row) is NOT
    /// returned on a first call.
    ///
    /// The SELECT + UPSERT pair runs under one connection lock, so
    /// concurrent calls for the same pair serialise — no double-
    /// delivery, no torn watermark.
    pub async fn read_new_messages(
        &self,
        caller_agent_instance_hierarchy: &str,
        spawned_agent_instance_hierarchy: &str,
    ) -> Result<Vec<MessageRow>, super::super::Error> {
        let conn = Arc::clone(&self.inner.conn);
        let caller = caller_agent_instance_hierarchy.to_string();
        let spawned = spawned_agent_instance_hierarchy.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<MessageRow>, super::super::Error> {
            use rusqlite::OptionalExtension as _;
            let conn = conn.lock().expect("filesystem db mutex poisoned");
            // 1. Current watermark for the pair, or 0 if no row.
            let watermark: i64 = conn
                .query_row(
                    "SELECT \"index\" FROM messages_queue \
                     WHERE caller_agent_instance_hierarchy = ?1 AND spawned_agent_instance_hierarchy = ?2",
                    rusqlite::params![&caller, &spawned],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .unwrap_or(0);
            // 2. Fetch unread rows.
            let mut stmt = conn.prepare_cached(
                "SELECT kind, response_id, path, timestamp, \"index\" FROM messages \
                 WHERE agent_instance_hierarchy = ?1 AND \"index\" > ?2 ORDER BY \"index\"",
            )?;
            let rows = stmt
                .query_map(rusqlite::params![&spawned, watermark], |r| {
                    let kind_str: String = r.get(0)?;
                    let response_id: String = r.get(1)?;
                    let path: String = r.get(2)?;
                    let timestamp: i64 = r.get(3)?;
                    let index: i64 = r.get(4)?;
                    Ok((kind_str, response_id, path, timestamp, index))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            // Drop the prepared-statement borrow before doing the
            // subsequent INSERT — `prepare_cached` holds `&conn`.
            drop(stmt);
            // Map raw tuples → MessageRow (parsing kind happens here
            // so we can surface a typed error before the upsert).
            let rows: Vec<MessageRow> = rows
                .into_iter()
                .map(|(kind_str, response_id, path, ts, idx)| {
                    Ok(MessageRow {
                        agent_instance_hierarchy: spawned.clone(),
                        response_id,
                        kind: parse_message_kind(&kind_str)?,
                        path,
                        timestamp: ts.max(0) as u64,
                        index: idx.max(0) as u64,
                    })
                })
                .collect::<Result<Vec<_>, super::super::Error>>()?;
            // 3. Upsert watermark to max returned index when we got anything.
            if let Some(new_max) = rows.last().map(|r| r.index as i64) {
                conn.execute(
                    "INSERT INTO messages_queue (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy, \"index\") \
                     VALUES (?1, ?2, ?3) \
                     ON CONFLICT (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy) \
                     DO UPDATE SET \"index\" = excluded.\"index\"",
                    rusqlite::params![&caller, &spawned, new_max],
                )?;
            }
            Ok(rows)
        })
        .await
        .map_err(|e| {
            super::super::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?
    }

    /// Read every message row for `spawned_agent_instance_hierarchy` (no watermark
    /// filter) in ascending index order, then upsert
    /// `caller_agent_instance_hierarchy`'s watermark for the pair to the returned
    /// max index. Same atomicity guarantee as
    /// [`Self::read_new_messages`]: the SELECT + UPSERT run under
    /// one connection lock.
    ///
    /// The watermark only moves forward — MAX over every row is
    /// always ≥ whatever was previously stored, so a no-op when
    /// the caller has already drained.
    pub async fn read_all_messages(
        &self,
        caller_agent_instance_hierarchy: &str,
        spawned_agent_instance_hierarchy: &str,
    ) -> Result<Vec<MessageRow>, super::super::Error> {
        let conn = Arc::clone(&self.inner.conn);
        let caller = caller_agent_instance_hierarchy.to_string();
        let spawned = spawned_agent_instance_hierarchy.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<MessageRow>, super::super::Error> {
            let conn = conn.lock().expect("filesystem db mutex poisoned");
            let mut stmt = conn.prepare_cached(
                "SELECT kind, response_id, path, timestamp, \"index\" FROM messages \
                 WHERE agent_instance_hierarchy = ?1 ORDER BY \"index\"",
            )?;
            let rows = stmt
                .query_map(rusqlite::params![&spawned], |r| {
                    let kind_str: String = r.get(0)?;
                    let response_id: String = r.get(1)?;
                    let path: String = r.get(2)?;
                    let timestamp: i64 = r.get(3)?;
                    let index: i64 = r.get(4)?;
                    Ok((kind_str, response_id, path, timestamp, index))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(stmt);
            let rows: Vec<MessageRow> = rows
                .into_iter()
                .map(|(kind_str, response_id, path, ts, idx)| {
                    Ok(MessageRow {
                        agent_instance_hierarchy: spawned.clone(),
                        response_id,
                        kind: parse_message_kind(&kind_str)?,
                        path,
                        timestamp: ts.max(0) as u64,
                        index: idx.max(0) as u64,
                    })
                })
                .collect::<Result<Vec<_>, super::super::Error>>()?;
            if let Some(new_max) = rows.last().map(|r| r.index as i64) {
                conn.execute(
                    "INSERT INTO messages_queue (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy, \"index\") \
                     VALUES (?1, ?2, ?3) \
                     ON CONFLICT (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy) \
                     DO UPDATE SET \"index\" = excluded.\"index\"",
                    rusqlite::params![&caller, &spawned, new_max],
                )?;
            }
            Ok(rows)
        })
        .await
        .map_err(|e| {
            super::super::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?
    }

    /// Internal: ensure the agent's mutable state is initialised.
    /// Seeds `next_index` from `MAX(index) WHERE agent_instance_hierarchy = ?` + 1
    /// the first time this id is seen. Idempotent — losing-race
    /// callers see the winner's entry.
    async fn ensure_agent(
        &self,
        agent_instance_hierarchy: &str,
    ) -> Result<Arc<AgentEntry>, super::super::Error> {
        // Fast path.
        {
            let guard = self
                .inner
                .agents
                .lock()
                .expect("queue agents mutex poisoned");
            if let Some(entry) = guard.get(agent_instance_hierarchy) {
                return Ok(Arc::clone(entry));
            }
        }
        // Slow path: seed next_index via the blocking pool.
        let max = schema::max_index_async(
            Arc::clone(&self.inner.conn),
            agent_instance_hierarchy.to_string(),
        )
        .await?;
        let entry = Arc::new(AgentEntry {
            state: StdMutex::new(AgentMutableState {
                next_index: max.map(|m| m + 1).unwrap_or(0),
                request_inserted: false,
                inserted_paths: HashSet::new(),
            }),
        });
        let mut guard = self
            .inner
            .agents
            .lock()
            .expect("queue agents mutex poisoned");
        Ok(Arc::clone(
            guard.entry(agent_instance_hierarchy.to_string()).or_insert(entry),
        ))
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
