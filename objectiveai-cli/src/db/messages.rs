//! Shared per-agent-id API for the `messages` table.
//!
//! Concurrency note: per-agent state (next-index counter, request-once
//! flag, path-dedup set) is serialized through a per-agent
//! `std::sync::Mutex`. The pool itself is concurrency-safe — concurrent
//! callers no longer serialize through one `Mutex<Connection>` like the
//! sqlite predecessor did. Only callers touching the SAME agent's
//! state contend on that agent's mutex; postgres handles the rest.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};

use objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind;
use sqlx::Row as _;

use super::pending::PendingNotification;
use super::schema::{self, MessageRow, parse_message_kind};
use super::{Error, Pool};

/// Per-stream handle to the shared `messages` table API. Owns:
/// - the per-agent monotonic `next_index` counter,
/// - the per-agent "request row inserted" once-flag,
/// - the per-agent path-dedup set.
///
/// All db reads/writes flow through this type. `Clone` is cheap —
/// internal state is `Arc`-shared across clones, so the LogWriter, the
/// cli-stream writer task, and any future readers can hold their own
/// clone without contention beyond the per-agent mutex.
#[derive(Clone)]
pub struct Queue {
    inner: Arc<QueueInner>,
}

struct QueueInner {
    pool: Pool,
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
    /// The full unique tuple is `(response_id, kind, path)`; the
    /// HashMap key contributes `response_id`, this set adds the
    /// remaining `(kind, path)`. None of the pairwise subsets is
    /// unique on its own.
    inserted_paths: HashSet<(RequestMessageKind, String)>,
}

impl Queue {
    /// Build a Queue backed by the shared postgres pool. `logs_dir` is
    /// still needed for the notification file write.
    pub fn new(pool: Pool, logs_dir: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(QueueInner {
                pool,
                logs_dir: logs_dir.into(),
                agents: StdMutex::new(HashMap::new()),
            }),
        }
    }

    /// Reserve and return the next monotonic db index for an agent.
    /// Seeds the agent's entry from `MAX(index) WHERE
    /// agent_instance_hierarchy = ?` + 1 on first use.
    pub async fn reserve_index(
        &self,
        agent_instance_hierarchy: &str,
    ) -> Result<u64, Error> {
        let entry = self.ensure_agent(agent_instance_hierarchy).await?;
        let mut state = entry.state.lock().expect("agent state mutex poisoned");
        let idx = state.next_index;
        state.next_index += 1;
        Ok(idx)
    }

    /// Insert one row at a caller-given index. `response_id` is the
    /// bare agent-completion chunk id; stored in its own column so the
    /// reader doesn't have to parse it back out of `agent_instance_hierarchy`.
    pub async fn insert(
        &self,
        agent_instance_hierarchy: &str,
        response_id: &str,
        kind: RequestMessageKind,
        path: String,
        timestamp: u64,
        index: u64,
    ) -> Result<(), Error> {
        self.ensure_agent(agent_instance_hierarchy).await?;
        schema::insert(
            &self.inner.pool,
            agent_instance_hierarchy,
            response_id,
            kind,
            &path,
            timestamp,
            index,
        )
        .await
    }

    /// Insert the per-stream request row at most once per agent.
    /// Reserves the next index under the same lock so concurrent
    /// callers can't race past the dedup check. Returns `true` if the
    /// row was inserted, `false` if a prior call already did it.
    pub async fn insert_request_once(
        &self,
        agent_instance_hierarchy: &str,
        response_id: &str,
        kind: RequestMessageKind,
        path: String,
        timestamp: u64,
    ) -> Result<bool, Error> {
        let entry = self.ensure_agent(agent_instance_hierarchy).await?;
        let index = {
            let mut state = entry.state.lock().expect("agent state mutex poisoned");
            if state.request_inserted {
                return Ok(false);
            }
            state.request_inserted = true;
            let idx = state.next_index;
            state.next_index += 1;
            idx
        };
        schema::insert(
            &self.inner.pool,
            agent_instance_hierarchy,
            response_id,
            kind,
            &path,
            timestamp,
            index,
        )
        .await?;
        Ok(true)
    }

    /// Register a `(kind, path)` pair for dedup under
    /// `agent_instance_hierarchy`. Returns `true` if newly inserted,
    /// `false` if already present (caller should skip the insert).
    pub async fn register_path(
        &self,
        agent_instance_hierarchy: &str,
        kind: RequestMessageKind,
        path: &str,
    ) -> Result<bool, Error> {
        let entry = self.ensure_agent(agent_instance_hierarchy).await?;
        let mut state = entry.state.lock().expect("agent state mutex poisoned");
        Ok(state.inserted_paths.insert((kind, path.to_string())))
    }

    /// Reserve the agent's next index for a pending notification and
    /// return a [`PendingNotification`] handle the caller queues
    /// locally for a later [`Self::insert_notification`] call.
    ///
    /// The pre-postgres writer extracted the content into per-leaf
    /// log files and emitted a `RichContentLog` envelope alongside;
    /// the postgres-backed writer hasn't yet wired notification
    /// content into the content tables, so this just reserves the
    /// index and stamps a bare-index path.
    pub async fn reserve_pending_notification(
        &self,
        agent_instance_hierarchy: &str,
        response_id: &str,
    ) -> Result<PendingNotification, Error> {
        let entry = self.ensure_agent(agent_instance_hierarchy).await?;
        let index = {
            let mut state = entry.state.lock().expect("agent state mutex poisoned");
            let idx = state.next_index;
            state.next_index += 1;
            idx
        };
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
    ) -> Result<(), Error> {
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

    /// Read every message for `spawned_agent_instance_hierarchy` whose
    /// `index` is strictly greater than `caller_agent_instance_hierarchy`'s
    /// watermark in `messages_queue`, then upsert the watermark to the
    /// max returned index. Returns the matching `MessageRow`s in
    /// ascending index order.
    ///
    /// First-read semantics: when no `messages_queue` row exists yet,
    /// the watermark defaults to `0`. The query is strict `>`, so
    /// `index = 0` (typically the request row) is NOT returned on a
    /// first call.
    ///
    /// The SELECT + UPSERT pair runs inside one transaction, so
    /// concurrent calls for the same pair serialise — no double-
    /// delivery, no torn watermark.
    pub async fn read_new_messages(
        &self,
        caller_agent_instance_hierarchy: &str,
        spawned_agent_instance_hierarchy: &str,
    ) -> Result<Vec<MessageRow>, Error> {
        let mut tx = self.inner.pool.begin().await?;

        // 1. Current watermark for the pair, or 0 if no row.
        let watermark: i64 = sqlx::query_scalar(
            "SELECT \"index\" FROM messages_queue \
             WHERE caller_agent_instance_hierarchy = $1 \
               AND spawned_agent_instance_hierarchy = $2",
        )
        .bind(caller_agent_instance_hierarchy)
        .bind(spawned_agent_instance_hierarchy)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(0i64);

        // 2. Fetch unread rows.
        let rows = sqlx::query(
            "SELECT kind, response_id, path, timestamp, \"index\" FROM messages \
             WHERE agent_instance_hierarchy = $1 AND \"index\" > $2 ORDER BY \"index\"",
        )
        .bind(spawned_agent_instance_hierarchy)
        .bind(watermark)
        .fetch_all(&mut *tx)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let kind_str: String = row.try_get(0)?;
            let response_id: String = row.try_get(1)?;
            let path: String = row.try_get(2)?;
            let timestamp: i64 = row.try_get(3)?;
            let index: i64 = row.try_get(4)?;
            out.push(MessageRow {
                agent_instance_hierarchy: spawned_agent_instance_hierarchy.to_string(),
                response_id,
                kind: parse_message_kind(&kind_str)?,
                path,
                timestamp: timestamp.max(0) as u64,
                index: index.max(0) as u64,
            });
        }

        // 3. Upsert watermark to max returned index when we got
        //    anything.
        if let Some(new_max) = out.last().map(|r| r.index as i64) {
            sqlx::query(
                "INSERT INTO messages_queue \
                 (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy, \"index\") \
                 VALUES ($1, $2, $3) \
                 ON CONFLICT (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy) \
                 DO UPDATE SET \"index\" = EXCLUDED.\"index\"",
            )
            .bind(caller_agent_instance_hierarchy)
            .bind(spawned_agent_instance_hierarchy)
            .bind(new_max)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(out)
    }

    /// Read every message row for `spawned_agent_instance_hierarchy`
    /// (no watermark filter) in ascending index order, then upsert
    /// `caller_agent_instance_hierarchy`'s watermark for the pair to
    /// the returned max index. The watermark only moves forward.
    pub async fn read_all_messages(
        &self,
        caller_agent_instance_hierarchy: &str,
        spawned_agent_instance_hierarchy: &str,
    ) -> Result<Vec<MessageRow>, Error> {
        let mut tx = self.inner.pool.begin().await?;

        let rows = sqlx::query(
            "SELECT kind, response_id, path, timestamp, \"index\" FROM messages \
             WHERE agent_instance_hierarchy = $1 ORDER BY \"index\"",
        )
        .bind(spawned_agent_instance_hierarchy)
        .fetch_all(&mut *tx)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let kind_str: String = row.try_get(0)?;
            let response_id: String = row.try_get(1)?;
            let path: String = row.try_get(2)?;
            let timestamp: i64 = row.try_get(3)?;
            let index: i64 = row.try_get(4)?;
            out.push(MessageRow {
                agent_instance_hierarchy: spawned_agent_instance_hierarchy.to_string(),
                response_id,
                kind: parse_message_kind(&kind_str)?,
                path,
                timestamp: timestamp.max(0) as u64,
                index: index.max(0) as u64,
            });
        }

        if let Some(new_max) = out.last().map(|r| r.index as i64) {
            sqlx::query(
                "INSERT INTO messages_queue \
                 (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy, \"index\") \
                 VALUES ($1, $2, $3) \
                 ON CONFLICT (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy) \
                 DO UPDATE SET \"index\" = EXCLUDED.\"index\"",
            )
            .bind(caller_agent_instance_hierarchy)
            .bind(spawned_agent_instance_hierarchy)
            .bind(new_max)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(out)
    }

    /// Ensure the agent's mutable state is initialised. Seeds
    /// `next_index` from `MAX(index) WHERE agent_instance_hierarchy = ?`
    /// + 1 the first time this id is seen. Idempotent — losing-race
    /// callers see the winner's entry.
    async fn ensure_agent(
        &self,
        agent_instance_hierarchy: &str,
    ) -> Result<Arc<AgentEntry>, Error> {
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
        // Slow path: seed next_index. Inline the schema query to keep
        // the call site explicit about the pool boundary.
        let max =
            schema::max_index(&self.inner.pool, agent_instance_hierarchy).await?;
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
            guard
                .entry(agent_instance_hierarchy.to_string())
                .or_insert(entry),
        ))
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
