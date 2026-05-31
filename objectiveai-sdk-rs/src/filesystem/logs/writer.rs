use std::collections::HashMap;
use std::path::PathBuf;

use futures::stream::{FuturesUnordered, StreamExt};

use crate::agent::completions::message::RichContent;
use crate::agent::completions::response::streaming::AgentCompletionIds;

use super::super::db::messages::Queue;
use super::super::db::pending::PendingNotification;
use super::super::db::schema::{MessageKind, MessageRow};
use super::LogFile;

/// Function-pointer signature for `produce_message_rows()` erased
/// across chunk types. The returned iterator borrows from the chunk;
/// the `for<'a>` lifetime keeps the pointer monomorphic.
pub type ProduceRows<C> =
    for<'a> fn(&'a C) -> Box<dyn Iterator<Item = MessageRow> + Send + 'a>;

/// Deferred request-side file producer attached via
/// [`LogWriter::with_request`]. The closure captures the cloned
/// request and runs at first-chunk flush time, when the response id
/// is known.
struct PendingRequest {
    /// Route prefix for the request log tree, e.g.
    /// `"agents/completions/request"`. Retained so the writer can
    /// match the produced summary file against its expected location
    /// (currently unused for matching, kept for future tooling).
    #[allow(dead_code)]
    route: String,
    /// Walks the request, returns every [`LogFile`] under
    /// `<route>/...` plus the top-level summary file. The summary
    /// is conventionally the last element — see each
    /// [`super::ProducesRequestFiles`] impl.
    produce: Box<dyn FnOnce(&str) -> Vec<LogFile> + Send>,
}

/// Writes streaming chunks to the log file structure on disk, and
/// in parallel inserts request / assistant_response / tool_response /
/// agent_completion_notification rows into per-agent SQLite databases.
///
/// `C` is the chunk type. The `produce` function pointer extracts
/// [`LogFile`]s from each chunk; `produce_rows` extracts
/// [`MessageRow`]s (lazy iterator).
///
/// Maintains a buffer of previously written file contents so that
/// unchanged files are not rewritten on every chunk.
///
/// All per-agent SQLite state lives in the shared
/// [`Queue`](super::super::db::messages::Queue) handle. The writer holds a
/// clone of it and delegates every db operation through it.
pub struct LogWriter<C> {
    logs_dir: PathBuf,
    produce: fn(&C) -> Option<Vec<LogFile>>,
    primary_id: Option<String>,
    buffer: HashMap<String, Vec<u8>>,
    /// A deferred request-side file producer waiting on the response
    /// ID. Holds the route + a [`super::ProducesRequestFiles`]
    /// closure that, given the discovered id, walks the request and
    /// returns every on-disk [`LogFile`] (leaves plus the top-level
    /// summary). Cleared after the first chunk is written.
    pending_request: Option<PendingRequest>,
    /// Shared per-agent-id db API. `None` disables per-agent SQLite
    /// writes entirely.
    queue: Option<Queue>,
    /// `kind` for the per-agent request row. Inserted (at most once
    /// per agent) for every id surfaced by `agent_completion_ids()`.
    /// `None` skips the request row entirely (used for factories
    /// whose request kind isn't in the WORK.md list).
    request_kind: Option<MessageKind>,
    /// Function pointer that extracts [`MessageRow`]s from a chunk
    /// lazily. `None` disables row extraction even when `queue` is
    /// set (factories wire them as a pair, but this stays optional
    /// to keep `LogWriter` usable without DB writes).
    produce_rows: Option<ProduceRows<C>>,
    /// Path of the on-disk request log file (relative to `logs_dir`).
    /// Captured once on the first chunk so it can be reused as the
    /// `path` column for every agent's request row.
    request_file_path: Option<String>,
    /// Optional caller lineage prefix prepended to every
    /// `chunk.agent_completion_ids()` value before it lands in the
    /// `messages.agent_id` column. `None` keeps the bare form (the
    /// original behaviour) for callers that don't go through a cli
    /// boundary; `Some("cli")`, `Some("cli/parent-X")`, etc. for
    /// stamped runs. Disambiguates two agents that happen to share
    /// the same `chunk.id` under different callers.
    caller_agent_id: Option<String>,
    /// The most recently-received chunk, buffered awaiting a
    /// successor. Processed on the NEXT `write` call or by
    /// `finalize`. `None` before the first `write` and again once
    /// `finalize` flushes it. Defers all per-chunk work (files,
    /// DB rows, notification drain) by one chunk so a chunk's
    /// outputs only commit once a successor confirms the chunk is
    /// done evolving.
    pending_chunk: Option<C>,
}

impl<C> LogWriter<C> {
    pub fn new(
        logs_dir: PathBuf,
        produce: fn(&C) -> Option<Vec<LogFile>>,
    ) -> Self {
        Self {
            logs_dir,
            produce,
            primary_id: None,
            buffer: HashMap::new(),
            pending_request: None,
            queue: None,
            request_kind: None,
            produce_rows: None,
            request_file_path: None,
            caller_agent_id: None,
            pending_chunk: None,
        }
    }

    /// Stamp the caller's lineage onto every `messages.agent_id` this
    /// writer inserts. `Some("cli")` prepends `"cli/"`; `None` keeps
    /// the bare chunk-emitted id. Passed verbatim — slashes inside
    /// `caller` (e.g. nested-spawn case `"cli/parent-X"`) become
    /// real subdir segments when the pipe path is derived from the
    /// same lineage string elsewhere in cli-stream.
    pub fn with_caller_agent_id(mut self, caller: Option<String>) -> Self {
        self.caller_agent_id = caller;
        self
    }

    /// Apply [`Self::with_caller_agent_id`]'s lineage transform to
    /// a single chunk-derived `agent_id`. `None` caller passes through.
    fn lineage_agent_id(&self, raw: &str) -> String {
        match self.caller_agent_id.as_deref() {
            Some(c) => format!("{c}/{raw}"),
            None => raw.to_string(),
        }
    }

    /// Attach a request body that will be written alongside the first
    /// response chunk. The request itself is captured eagerly (cloned
    /// into a closure), but the on-disk extraction is deferred until
    /// the response ID is known — the per-leaf filenames embed the
    /// id, so we can't materialize anything until the first chunk
    /// arrives. The closure produces the full Log envelope
    /// (`<route>/<id>.json`) plus every extracted child (messages,
    /// response_format, continuation, …) using each request type's
    /// [`super::ProducesRequestFiles`] impl.
    pub fn with_request<R>(
        mut self,
        route: impl Into<String>,
        request: &R,
    ) -> Result<Self, super::super::Error>
    where
        R: super::ProducesRequestFiles + Clone + Send + 'static,
    {
        let route = route.into();
        let cloned = request.clone();
        let route_for_closure = route.clone();
        let produce: Box<dyn FnOnce(&str) -> Vec<LogFile> + Send> =
            Box::new(move |id: &str| {
                let (_, files) = cloned.produce_files(id, &route_for_closure);
                files
            });
        self.pending_request = Some(PendingRequest { route, produce });
        Ok(self)
    }

    /// Attach the shared per-agent-id [`Queue`] and a chunk-to-rows
    /// extractor. Sets the writer up to insert a request row of
    /// `request_kind` into every agent's db (discovered via
    /// `agent_completion_ids()`), and a row per `assistant_response`
    /// / `tool_response` observed in any chunk.
    pub fn with_queue(
        mut self,
        queue: Queue,
        request_kind: Option<MessageKind>,
        produce_rows: ProduceRows<C>,
    ) -> Self {
        self.queue = Some(queue);
        self.request_kind = request_kind;
        self.produce_rows = Some(produce_rows);
        self
    }

    /// Borrow the writer's shared db handle. Useful for callers that
    /// need to enqueue notifications outside the chunk loop.
    pub fn queue(&self) -> Option<&Queue> {
        self.queue.as_ref()
    }

    /// The ID of the primary (root) log entry.
    ///
    /// Returns `None` until at least one chunk has been written.
    pub fn primary_id(&self) -> Option<&str> {
        self.primary_id.as_deref()
    }

    /// Reserve the agent's next db index, write the notification log
    /// file immediately, and return a [`PendingNotification`] handle
    /// the caller queues locally. Delegates to [`Queue::write_notification`].
    /// `response_id` is the target agent-completion's id (the same
    /// value `AgentCompletionNotifyParams.response_id` carries on
    /// the wire); the caller threads it down from the pipe binding.
    pub async fn write_notification(
        &mut self,
        agent_id: &str,
        response_id: &str,
        content: &RichContent,
    ) -> Result<PendingNotification, super::super::Error> {
        match &self.queue {
            Some(q) => q.write_notification(agent_id, response_id, content).await,
            None => Ok(PendingNotification {
                agent_id: agent_id.to_string(),
                response_id: response_id.to_string(),
                index: 0,
                path: String::new(),
                timestamp: now_secs(),
            }),
        }
    }

    /// Write a chunk to disk. Files whose content hasn't changed since the
    /// last write are skipped. All file writes plus all per-agent DB
    /// inserts (requests, messages, and any drained notifications)
    /// run concurrently — only operations targeting the same agent's
    /// db serialise (via that agent's mutex inside [`Queue`]).
    ///
    /// `pending` is the caller's local notification queue. For each
    /// tool-response row encountered, every queued notification with
    /// the matching `agent_id` is removed from `pending` and its
    /// `INSERT` is pushed into the same concurrent op set (at its
    /// already-reserved index — so the notification's index precedes
    /// the tool response's reserved index). Notifications for agents
    /// not in this chunk remain in `pending` for the next call.
    pub async fn write(
        &mut self,
        chunk: &C,
        pending: &mut Vec<PendingNotification>,
    ) -> Result<(), super::super::Error>
    where
        C: AgentCompletionIds + Clone,
    {
        // Process the previously-buffered chunk (one chunk behind),
        // then stash the current one to await its successor.
        let prev = self.pending_chunk.replace(chunk.clone());
        if let Some(buffered) = prev {
            self.process_chunk(&buffered, pending).await?;
        }
        Ok(())
    }

    /// Verbatim body of the original `write`. Splits files +
    /// DB rows + notification drain in one atomic op set.
    async fn process_chunk(
        &mut self,
        chunk: &C,
        pending: &mut Vec<PendingNotification>,
    ) -> Result<(), super::super::Error>
    where
        C: AgentCompletionIds,
    {
        let mut files = match (self.produce)(chunk) {
            Some(files) => files,
            None => return Ok(()),
        };

        // First-chunk: capture primary_id, flush the pending request
        // file tree alongside this chunk's files, and remember the
        // top-level request summary's path for per-agent request-row
        // inserts. Each `ProducesRequestFiles` impl appends the
        // summary last, so `files.last()` after the extend points at
        // `<route>/<id>.json`.
        if self.primary_id.is_none() {
            if let Some(last) = files.last() {
                self.primary_id = Some(last.id.clone());
                if let Some(pending) = self.pending_request.take() {
                    let request_files = (pending.produce)(&last.id.clone());
                    if let Some(summary) = request_files.last() {
                        self.request_file_path = Some(summary.path());
                    }
                    files.extend(request_files);
                }
            }
        }

        // Filter out files whose content matches the buffer.
        let changed: Vec<LogFile> = files
            .into_iter()
            .filter(|file| {
                let path = file.path();
                if self
                    .buffer
                    .get(&path)
                    .map_or(false, |prev| prev == &file.content)
                {
                    return false;
                }
                self.buffer.insert(path, file.content.clone());
                true
            })
            .collect();

        // Build the concurrent op set: file writes + per-agent request
        // rows + per-message rows + drained notification rows.
        let mut ops: FuturesUnordered<
            std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<(), super::super::Error>>
                        + Send,
                >,
            >,
        > = FuturesUnordered::new();

        // File writes.
        for file in changed {
            let logs_dir = self.logs_dir.clone();
            ops.push(Box::pin(async move {
                let full_path = logs_dir.join(file.path());
                if let Some(parent) = full_path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        super::super::Error::Write(parent.to_path_buf(), e)
                    })?;
                }
                tokio::fs::write(&full_path, file.content)
                    .await
                    .map_err(|e| super::super::Error::Write(full_path, e))
            }));
        }

        // SQLite ops, if a queue is wired.
        if let Some(queue) = self.queue.clone() {
            // 1. Per-agent request rows. One per newly-seen agent.
            if let (Some(kind), Some(req_path)) =
                (self.request_kind, self.request_file_path.clone())
            {
                let now = now_secs();
                // Lineage-stamp every chunk-emitted id with the
                // caller prefix so two agents that happen to share
                // `chunk.id` under different callers can't collide
                // in `messages.agent_id`.
                // (agent_id, response_id) pairs — `raw` is the bare
                // response id straight from the chunk; lineage-stamping
                // produces the column value. We thread both so the
                // reader doesn't have to reverse the stamp.
                let id_pairs: Vec<(String, String)> = chunk
                    .agent_completion_ids()
                    .map(|raw| (self.lineage_agent_id(raw), raw.to_string()))
                    .collect();
                for (agent_id, response_id) in id_pairs {
                    let queue = queue.clone();
                    let req_path = req_path.clone();
                    ops.push(Box::pin(async move {
                        queue
                            .insert_request_once(
                                &agent_id,
                                &response_id,
                                kind,
                                req_path,
                                now,
                            )
                            .await
                            .map(|_| ())
                    }));
                }
            }

            // 2. Per-message rows + per-tool-response notification drain.
            if let Some(rows_fn) = self.produce_rows {
                let rows: Vec<MessageRow> = rows_fn(chunk)
                    .map(|mut row| {
                        row.agent_id = self.lineage_agent_id(&row.agent_id);
                        row
                    })
                    .collect();
                for row in rows {
                    // Dedup by (kind, path) via the queue — kind
                    // matters because assistant and tool messages can
                    // share the same `path` (the bare index) but
                    // dispatch to different parsers on read.
                    let inserted = queue
                        .register_path(&row.agent_id, row.kind, &row.path)
                        .await?;
                    if !inserted {
                        continue;
                    }

                    // Tool-response rows drain the matching pending
                    // notifications FIRST. Drained notifs insert at
                    // their reserved (earlier) indices; the tool
                    // response then reserves and inserts at its own
                    // (later) index.
                    if matches!(row.kind, MessageKind::ToolResponse) {
                        let agent = row.agent_id.clone();
                        let mut i = 0;
                        while i < pending.len() {
                            if pending[i].agent_id == agent
                                && !pending[i].path.is_empty()
                            {
                                let notif = pending.remove(i);
                                let queue = queue.clone();
                                ops.push(Box::pin(async move {
                                    queue.insert_notification(notif).await
                                }));
                            } else {
                                i += 1;
                            }
                        }
                    }

                    let index = queue.reserve_index(&row.agent_id).await?;
                    let queue = queue.clone();
                    let path = row.path;
                    let kind = row.kind;
                    let ts = row.timestamp;
                    let agent_id = row.agent_id;
                    let response_id = row.response_id;
                    ops.push(Box::pin(async move {
                        queue
                            .insert(&agent_id, &response_id, kind, path, ts, index)
                            .await
                    }));
                }
            }
        }

        // Drive everything to completion. First error short-circuits;
        // remaining futures are dropped as the FuturesUnordered drops.
        while let Some(result) = ops.next().await {
            result?;
        }
        Ok(())
    }

    /// Flush the buffered last chunk (if any), then drain any
    /// remaining notifications into their respective per-agent dbs.
    /// Called by the cli-stream writer task after the chunk channel
    /// closes and any in-flight notifications have been pulled off
    /// the wire — this is the writer's only "stream is over" signal,
    /// and it's where the deferred-by-one `pending_chunk` finally
    /// gets processed. Each surviving notification is inserted at
    /// its already-reserved index.
    pub async fn finalize(
        &mut self,
        pending: &mut Vec<PendingNotification>,
    ) -> Result<(), super::super::Error>
    where
        C: AgentCompletionIds,
    {
        // Flush the last buffered chunk before doing the
        // notification drain — its tool-response rows may want to
        // drain notifications too, and those should land first.
        if let Some(buffered) = self.pending_chunk.take() {
            self.process_chunk(&buffered, pending).await?;
        }
        let queue = match self.queue.clone() {
            Some(q) => q,
            None => {
                pending.clear();
                return Ok(());
            }
        };
        let mut ops: FuturesUnordered<
            std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<(), super::super::Error>>
                        + Send,
                >,
            >,
        > = FuturesUnordered::new();
        for notif in pending.drain(..) {
            if notif.path.is_empty() {
                continue;
            }
            let queue = queue.clone();
            ops.push(Box::pin(async move {
                queue.insert_notification(notif).await
            }));
        }
        while let Some(result) = ops.next().await {
            result?;
        }
        Ok(())
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
