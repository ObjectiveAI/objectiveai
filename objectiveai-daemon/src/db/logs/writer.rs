//! `LogWriter<C>` — postgres log writer fronted by an mpsc sender.
//!
//! Architecturally:
//!
//! - The constructor (`write_agent_completion` etc.) spawns a tokio
//!   task that owns the [`LogWriterState`] and the
//!   `mpsc::UnboundedReceiver<C>`. It returns the [`LogWriter`]
//!   handle plus a [`tokio::sync::oneshot::Receiver<String>`] that
//!   fires the first time the writer learns the stream's primary
//!   `response_id`.
//! - [`LogWriter::write`] is **synchronous** — it just hands the
//!   chunk to the listener task via `UnboundedSender::send`. The
//!   caller's chunk-yield hot path stays off the DB write critical
//!   path.
//! - The listener task `.push()`-folds every chunk into one
//!   stream-wide accumulator and runs the persistence logic
//!   ([`LogWriterState::apply_chunk`]) against that cumulative
//!   aggregate — not a per-batch slice. Draining the queue per loop
//!   iteration only collapses how often the pass runs. Folding is
//!   correct because each tier's chunk is a cumulative roll-up of
//!   state — `push` folds the later chunk's deltas into the earlier
//!   one's accumulators (`AgentCompletionChunk::push` /
//!   `VectorCompletionChunk::push` / `FunctionExecutionChunk::push`) —
//!   so a row whose fields stream across several wire chunks (a tool
//!   call's id/name/arguments, streamed content text) is always
//!   persisted from its complete body, never a partial fragment.
//! - [`LogWriter::finalize`] consumes the writer by value, drops the
//!   sender, and `.await`s the JoinHandle. By the time it returns,
//!   both invariants hold: the channel is empty (sender dropped →
//!   `recv()` returned `None` only after every queued chunk was
//!   consumed) and the task's future has fully completed (no
//!   in-flight row-bucket joins or blob writes).
//!
//! Persistence pass (run per drain batch against the cumulative
//! accumulator):
//!
//! 1. **First pass**: capture the accumulator's `response_id`, INSERT
//!    the request blob (no `agent_instance_hierarchy` on the blob —
//!    that linkage lives in `objectiveai.messages`).
//! 2. **Every pass**: walk `chunk_rows(acc)` over the cumulative
//!    aggregate, gate each yielded [`RowValue`] through the shadow
//!    (Skip path is pure-memory — unchanged rows cost nothing), bucket
//!    the survivors by `agent_instance_hierarchy`. For every agent the
//!    writer hasn't seen yet in this stream's lifetime, prepend a
//!    `objectiveai.messages` row that registers the request blob in
//!    that agent's history.
//! 3. **Per-bucket execution**: rows within one agent's bucket fire
//!    sequentially (so the per-agent ORDER BY `"index"` matches the
//!    iterator's order). All buckets fire concurrently via
//!    `try_join_all`.
//!
//! The response blob is written separately, exactly once, by
//! `listener_loop` after the last chunk — from the same cumulative
//! accumulator, so blob and rows can never disagree.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use std::pin::Pin;

use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AgentCompletionIds,
};
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
use objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams;
use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;
use serde::Serialize;
use tokio::sync::{Mutex, mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::db::Pool;

use super::row::{RowValue, WriterItem, WriterItems};
use super::rows::{
    agent_completion_chunk_rows, function_execution_chunk_rows,
    vector_completion_chunk_rows,
};
use super::shadow::{Shadow, WriteOp};
use super::write::{
    Tier, insert_request_blob, insert_request_messages_row, insert_response_blob,
    update_agent_token_usage, write_value,
};

pub trait WriterChunk {
    fn primary_id(&self) -> &str;
    /// The spawned agent's AIH for the agent tier — where the writer
    /// derives request_message rows + the agent_ref from the REQUEST
    /// (the response chunk no longer carries them). `None` for the
    /// vector/function tiers, which surface these through the chunk
    /// stream instead.
    fn agent_tier_aih(&self) -> Option<&str> {
        None
    }
}

impl WriterChunk for AgentCompletionChunk {
    fn primary_id(&self) -> &str {
        self.id.as_str()
    }
    fn agent_tier_aih(&self) -> Option<&str> {
        Some(self.agent_instance_hierarchy.as_str())
    }
}
impl WriterChunk for VectorCompletionChunk {
    fn primary_id(&self) -> &str {
        self.id.as_str()
    }
}
impl WriterChunk for FunctionExecutionChunk {
    fn primary_id(&self) -> &str {
        self.id.as_str()
    }
}

/// Chunks that can surface `(AIH, definition source)` pairs for the
/// `objectiveai.agent_refs` registry: every nested agent completion
/// carrying `agent_inline` (each completion's FIRST chunk). Per the
/// registry's rule, `agent_remote` present → the remote wins;
/// otherwise the inline spec itself.
pub trait ChunkAgentRefs {
    fn collect_agent_refs(
        &self,
        out: &mut Vec<(String, crate::db::agent_refs::AgentRefValue)>,
    );
}

impl ChunkAgentRefs for AgentCompletionChunk {
    fn collect_agent_refs(
        &self,
        _out: &mut Vec<(String, crate::db::agent_refs::AgentRefValue)>,
    ) {
        // The inner agent chunk no longer carries `agent_inline` (it
        // moved to the vector wrapper). Agent-tier refs are derived
        // from the REQUEST by the writer, not from the response chunk;
        // this impl is a no-op (used only when the chunk appears as a
        // standalone agent completion or a function reasoning summary).
    }
}

impl ChunkAgentRefs for VectorCompletionChunk {
    fn collect_agent_refs(
        &self,
        out: &mut Vec<(String, crate::db::agent_refs::AgentRefValue)>,
    ) {
        // `agent_inline` now lives on the per-agent vector wrapper, not
        // the inner agent chunk. Pair it with the inner chunk's
        // AIH / remote.
        for completion in &self.completions {
            if let Some(inline) = &completion.agent_inline {
                let value = match &completion.inner.agent_remote {
                    Some(remote) => {
                        crate::db::agent_refs::AgentRefValue::remote(remote)
                    }
                    None => crate::db::agent_refs::AgentRefValue::inline(inline),
                };
                if let Some(value) = value {
                    out.push((
                        completion.inner.agent_instance_hierarchy.clone(),
                        value,
                    ));
                }
            }
        }
    }
}

impl ChunkAgentRefs for FunctionExecutionChunk {
    fn collect_agent_refs(
        &self,
        out: &mut Vec<(String, crate::db::agent_refs::AgentRefValue)>,
    ) {
        use objectiveai_sdk::functions::executions::response::streaming::TaskChunk;
        for task in &self.tasks {
            match task {
                TaskChunk::FunctionExecution(wrapper) => {
                    wrapper.inner.collect_agent_refs(out);
                }
                TaskChunk::VectorCompletion(wrapper) => {
                    wrapper.inner.collect_agent_refs(out);
                }
            }
        }
        if let Some(reasoning) = &self.reasoning {
            reasoning.inner.collect_agent_refs(out);
        }
    }
}

/// CLI-side wrapper exposing the SDK's intrinsic `push(&mut self,
/// other: &Self)` method via a uniform trait. Each impl simply
/// delegates to the chunk type's inherent method — the SDK already
/// guarantees `push` is a correct accumulator for the tier's
/// cumulative-state semantics.
pub trait ChunkPush {
    fn push(&mut self, other: &Self);
}

impl ChunkPush for AgentCompletionChunk {
    fn push(&mut self, other: &Self) {
        AgentCompletionChunk::push(self, other);
    }
}
impl ChunkPush for VectorCompletionChunk {
    fn push(&mut self, other: &Self) {
        VectorCompletionChunk::push(self, other);
    }
}
impl ChunkPush for FunctionExecutionChunk {
    fn push(&mut self, other: &Self) {
        FunctionExecutionChunk::push(self, other);
    }
}

/// Background-task-fronted log writer.
///
/// Construction (via `write_agent_completion` etc.) spawns a tokio
/// task that owns the per-stream [`LogWriterState`]. The handle here
/// is a thin sender + JoinHandle pair.
pub struct LogWriter<C> {
    tx: mpsc::UnboundedSender<C>,
    handle: JoinHandle<Result<(), crate::error::Error>>,
    /// Toggled `false` → `true` by the listener task once it has
    /// completed a single successful `apply_chunk`. Powers
    /// [`LogWriter::written_once`] (sync peek) and
    /// [`LogWriter::wait_written_once`] (async wait).
    written_rx: watch::Receiver<bool>,
    /// Mid-stream failure handoff slot shared with the listener —
    /// see [`finalize_with_stream_error`](Self::finalize_with_stream_error).
    stream_error: Arc<Mutex<Option<serde_json::Value>>>,
    _chunk: PhantomData<fn() -> C>,
}

impl<C> LogWriter<C> {
    /// Hand off one chunk to the listener task. Returns
    /// `Err(Error::Instance(_))` only when the listener has already
    /// exited — typically because an earlier DB write failed. The
    /// caller should treat that error the same way it would treat an
    /// upstream stream error: stop reading, surface upward.
    pub fn write(&self, chunk: C) -> Result<(), crate::error::Error> {
        self.tx
            .send(chunk)
            .map_err(|_| crate::error::Error::Instance(
                "log writer task has exited (earlier write failed)".to_string(),
            ))
    }

    /// Sync peek: has the listener completed at least one successful
    /// `apply_chunk` batch? Flips `false → true` exactly once,
    /// immediately after the first batch's write completes and
    /// before the listener parks on the next `recv`.
    pub fn written_once(&self) -> bool {
        *self.written_rx.borrow()
    }

    /// Async wait that resolves once the listener has completed its
    /// first successful `apply_chunk` batch. Returns immediately if
    /// that already happened. Errors only if the listener task
    /// exited before its first successful write (DB error on the
    /// very first batch).
    pub async fn wait_written_once(&self) -> Result<(), crate::error::Error> {
        let mut rx = self.written_rx.clone();
        rx.wait_for(|b| *b)
            .await
            .map(|_| ())
            .map_err(|_| crate::error::Error::Instance(
                "log writer task exited before completing its first write".to_string(),
            ))
    }

    /// Consume the writer. Drops the sender (signaling EOF to the
    /// listener) and awaits the task. Returns only once:
    ///
    /// - the channel is empty: `recv()` returns `None` only after the
    ///   listener has drained every queued chunk, AND
    /// - no work is in flight: the task's future has fully completed,
    ///   so no row-bucket joins or blob writes remain pending.
    ///
    /// Surfaces the first DB error the task encountered, if any.
    pub async fn finalize(self) -> Result<(), crate::error::Error> {
        let LogWriter { tx, handle, .. } = self;
        drop(tx);
        match handle.await {
            Ok(inner) => inner,
            Err(e) => Err(crate::error::Error::Instance(
                format!("log writer task: {e}"),
            )),
        }
    }

    /// [`finalize`](Self::finalize), carrying the caller's mid-stream
    /// failure. When `error` is `Some`, the listener — AFTER draining
    /// every queued chunk (so error rows land last, never out of
    /// order) — logs it as an `error` row for every nested agent
    /// completion that has NOT finished (no `usage` yet) and does not
    /// carry its own in-band error. Completions that finished cleanly
    /// get nothing: they started and ended with no errors.
    pub async fn finalize_with_stream_error(
        self,
        error: Option<serde_json::Value>,
    ) -> Result<(), crate::error::Error> {
        if let Some(value) = error {
            *self.stream_error.lock().await = Some(value);
        }
        self.finalize().await
    }
}

/// All the per-stream state the listener task owns. Was previously
/// inlined onto `LogWriter`; now lives entirely inside the spawned
/// task so the handle stays send-and-clone-cheap.
struct LogWriterState<C> {
    pool: Pool,
    tier: Tier,
    request_body: serde_json::Value,
    /// Agent tier only: the request's typed messages, unpacked into
    /// request_message content rows once on the first chunk. `None`
    /// for the vector/function tiers (their request_message rows ride
    /// the chunk stream). Immutable — written once, not shadow-gated.
    request_messages: Option<Vec<objectiveai_sdk::agent::completions::message::Message>>,
    /// Agent tier only: the agent-ref derived from the REQUEST's
    /// `agent` field, upserted once on the first chunk. Taken (moved)
    /// then. `None` for the other tiers (their refs come from the
    /// response via `ChunkAgentRefs`).
    request_agent_ref: Option<crate::db::agent_refs::AgentRefValue>,
    /// AIH of the caller who issued the request that spawned this
    /// writer (pulled from `ctx.config.agent_instance_hierarchy` at
    /// `spawn_writer` time). Written into the request blob row at
    /// `insert_request_blob` time. Constant for the writer's
    /// lifetime — one request = one sender.
    sender_agent_instance_hierarchy: String,
    /// Walks a chunk into [`WriterItem`]s — content rows AND, at each
    /// nested agent completion with usage, a per-AIH `total_tokens`
    /// snapshot. One traversal covers both.
    items_fn: for<'a> fn(&'a C) -> WriterItems<'a>,
    /// Last `total_tokens` written per AIH — dedups redundant
    /// overwrites across the writer's repeated passes over the
    /// cumulative accumulator.
    last_usage: HashMap<String, u64>,
    primary_id: Option<String>,
    /// Per-streaming-content-row shadow. Skip path is allocation-free.
    shadow: Shadow,
    /// Every `agent_instance_hierarchy` we've observed in this
    /// stream's lifetime. The first time an agent appears in the row
    /// iterator we insert a `objectiveai.messages` row registering the
    /// request blob in that agent's history; subsequent ticks see
    /// the agent already-marked and skip the registration.
    seen_agents: HashSet<String>,
    /// Live-conversation tee: every row the shadow admits
    /// (`Insert`/`Update`) is also shipped, full-value, to the resident
    /// daemon for `/agents/instances/{*aih}` fan-out. Best-effort and
    /// non-blocking — see [`super::tee`]. `None` = no tee (e.g. hand
    /// -built writers).
    tee: Option<super::tee::ConversationTee>,
    /// The tee's stateful row→typed-event mapper (head memory).
    frame_mapper: super::tee::FrameMapper,
    /// `(aih, response_id)` pairs whose in-band completion error has
    /// already been persisted — the cumulative accumulator re-yields
    /// the error item every tick once set. Also consulted by the
    /// post-drain mid-stream failure sweep.
    logged_errors: HashSet<(String, String)>,
    /// Per-tier walker over the folded chunk's nested agent
    /// completions — the mid-stream failure sweep's enumeration.
    statuses_fn: for<'a> fn(&'a C) -> super::rows::CompletionStatuses<'a>,
    /// The chunk's own TOP-LEVEL in-band error (a function execution
    /// failing on the wire without a transport error). `None`-returning
    /// for the agent tier — spawn's `note_error` owns that tier's
    /// stream errors.
    chunk_error_fn: for<'a> fn(&'a C) -> Option<&'a objectiveai_sdk::error::ResponseError>,
    /// Mid-stream failure handoff: the caller's stream error, set by
    /// [`LogWriter::finalize_with_stream_error`] BEFORE the EOF signal,
    /// consumed by the listener's post-drain sweep.
    stream_error: Arc<Mutex<Option<serde_json::Value>>>,
    _chunk: PhantomData<fn() -> C>,
}

impl<C> LogWriterState<C> {
    fn new(
        pool: Pool,
        tier: Tier,
        request_body: serde_json::Value,
        sender_agent_instance_hierarchy: String,
        items_fn: for<'a> fn(&'a C) -> WriterItems<'a>,
        request_messages: Option<
            Vec<objectiveai_sdk::agent::completions::message::Message>,
        >,
        request_agent_ref: Option<crate::db::agent_refs::AgentRefValue>,
        tee: Option<super::tee::ConversationTee>,
        statuses_fn: for<'a> fn(&'a C) -> super::rows::CompletionStatuses<'a>,
        chunk_error_fn: for<'a> fn(&'a C) -> Option<&'a objectiveai_sdk::error::ResponseError>,
        stream_error: Arc<Mutex<Option<serde_json::Value>>>,
    ) -> Self {
        Self {
            pool,
            tier,
            request_body,
            request_messages,
            request_agent_ref,
            sender_agent_instance_hierarchy,
            items_fn,
            last_usage: HashMap::new(),
            primary_id: None,
            shadow: Shadow::new(),
            seen_agents: HashSet::new(),
            tee,
            frame_mapper: super::tee::FrameMapper::default(),
            logged_errors: HashSet::new(),
            statuses_fn,
            chunk_error_fn,
            stream_error,
            _chunk: PhantomData,
        }
    }

    /// Agent tier: write the request's messages as request_message
    /// content rows, once, on the first chunk — before any response
    /// row so they read first. Immutable, so a direct one-time INSERT
    /// per row (no shadow). `response_id`/`aih` are the spawned agent
    /// completion's own id + AIH.
    async fn write_agent_request_messages(
        &mut self,
        response_id: &str,
        aih: &str,
    ) -> Result<(), crate::error::Error> {
        let Some(messages) = &self.request_messages else {
            return Ok(());
        };
        let ts = now_secs() as i64;
        for row in super::rows::request_message_rows(response_id, aih, messages) {
            // These rows bypass the shadow (immutable, written once),
            // so they must tee here or live subscribers never see the
            // spawned agent's opening messages.
            if let Some(tee) = &self.tee {
                if let Some(frame) = self.frame_mapper.map(&row, ts) {
                    tee.send(frame);
                }
            }
            write_value(&self.pool, WriteOp::Insert, &row, ts).await?;
        }
        Ok(())
    }

    /// Persist the cumulative aggregate's streaming-content rows
    /// (`listener_loop` hands in the stream-wide accumulator, not a
    /// per-batch slice, so every row is seen with its complete body).
    /// The response_id and request blob are established by
    /// `listener_loop` before the first call; the response blob is
    /// written by `listener_loop` after the last chunk. This method
    /// only touches the per-row content tables (gated through the
    /// shadow, so re-walking the unchanged majority is free) and the
    /// per-agent `objectiveai.messages` bookkeeping.
    async fn apply_chunk(&mut self, chunk: &C) -> Result<(), crate::error::Error>
    where
        C: Send + Sync,
    {
        let response_id = self
            .primary_id
            .clone()
            .expect("primary_id set by listener_loop before apply_chunk");
        let created_at_seed = now_secs() as i64;

        // One traversal of the chunk. Content rows are gated via the
        // shadow and bucketed by agent_instance_hierarchy (Vec inside
        // the HashMap preserves iterator order so per-bucket sequential
        // awaits match the walk's ordering). Usage items (per nested
        // agent completion carrying a non-`None` usage) overwrite the
        // per-AIH `total_tokens` snapshot inline — `last_usage` dedups
        // so the repeated passes over the cumulative accumulator only
        // write when the value changes.
        let mut buckets: HashMap<&str, Vec<(WriteOp, RowValue<'_>)>> = HashMap::new();
        for item in (self.items_fn)(chunk) {
            match item {
                WriterItem::Row(value) => {
                    let key = value.agent_instance_hierarchy();
                    match self.shadow.record(&value) {
                        WriteOp::Skip => {}
                        op => {
                            // Live-conversation tee: ship the admitted
                            // row (full current value) BEFORE its SQL
                            // runs — sequential walk order, never gated
                            // on DB latency. Best-effort, non-blocking.
                            if let Some(tee) = &self.tee {
                                if let Some(frame) =
                                    self.frame_mapper.map(&value, created_at_seed)
                                {
                                    tee.send(frame);
                                }
                            }
                            buckets.entry(key).or_default().push((op, value));
                        }
                    }
                }
                WriterItem::Usage { agent_instance_hierarchy, total_tokens } => {
                    if self.last_usage.get(agent_instance_hierarchy) != Some(&total_tokens) {
                        update_agent_token_usage(
                            &self.pool,
                            agent_instance_hierarchy,
                            total_tokens as i64,
                        )
                        .await?;
                        self.last_usage
                            .insert(agent_instance_hierarchy.to_string(), total_tokens);
                    }
                }
                WriterItem::Error { agent_instance_hierarchy, response_id: rid, error } => {
                    let logged_key =
                        (agent_instance_hierarchy.to_string(), rid.to_string());
                    if !self.logged_errors.contains(&logged_key) {
                        // Persist first, then tee — same order as the
                        // spawn-path `note_error`. Covers EVERY tier:
                        // nested agent completions inside vector /
                        // function executions flow through the same
                        // walker.
                        let value = serde_json::to_value(error)
                            .unwrap_or_else(|_| error.to_string().into());
                        super::errors::insert_error(
                            &self.pool,
                            agent_instance_hierarchy,
                            Some(rid),
                            &value,
                            created_at_seed,
                        )
                        .await?;
                        if let Some(tee) = &self.tee {
                            tee.send(super::tee::error_frame(
                                agent_instance_hierarchy.to_string(),
                                Some(rid.to_string()),
                                value,
                                created_at_seed,
                            ));
                        }
                        self.logged_errors.insert(logged_key);
                    }
                }
            }
        }

        // Build the per-agent bucket futures. Each future runs its
        // rows sequentially (order matters within one agent's
        // history); different agents run concurrently via
        // `try_join_all`. The seen_agents mutation happens
        // synchronously inside the map closure — by the time the
        // futures actually run, every bucket already knows whether it
        // owes a request-messages row.
        let pool = &self.pool;
        let tier = self.tier;
        let resp_id = response_id.as_str();
        let seen_agents = &mut self.seen_agents;
        let bucket_futures: Vec<
            Pin<Box<dyn Future<Output = Result<(), crate::error::Error>> + Send + '_>>,
        > = buckets
            .into_iter()
            .map(|(hier, items)| {
                let needs_request_row = !seen_agents.contains(hier);
                if needs_request_row {
                    seen_agents.insert(hier.to_string());
                }
                Box::pin(async move {
                    if needs_request_row {
                        insert_request_messages_row(
                            pool,
                            tier,
                            resp_id,
                            hier,
                            created_at_seed,
                        )
                        .await?;
                    }
                    for (op, value) in &items {
                        write_value(pool, *op, value, created_at_seed).await?;
                    }
                    Ok::<(), crate::error::Error>(())
                })
                    as Pin<
                        Box<
                            dyn Future<Output = Result<(), crate::error::Error>>
                                + Send
                                + '_,
                        >,
                    >
            })
            .collect();

        futures::future::try_join_all(bucket_futures).await?;

        Ok(())
    }

    /// Write the request blob exactly once, when `listener_loop` first
    /// learns the response_id (before any content row references it).
    /// The request blob carries no agent_instance_hierarchy — that
    /// linkage lives in `objectiveai.messages` (written per-agent in
    /// `apply_chunk`).
    async fn write_request_blob(
        &self,
        response_id: &str,
    ) -> Result<(), crate::error::Error> {
        let created_at_seed = now_secs() as i64;
        insert_request_blob(
            &self.pool,
            self.tier,
            response_id,
            &self.request_body,
            &self.sender_agent_instance_hierarchy,
            created_at_seed,
        )
        .await?;
        Ok(())
    }

    /// Write the complete response blob exactly once, from the
    /// cumulative aggregate of every chunk in the stream — built by
    /// `listener_loop` and handed in after the last chunk (finalize).
    /// A single INSERT: the blob is never a partial snapshot, so a
    /// chunk's tool-calls can't be lost to a per-batch overwrite.
    async fn write_response_blob(
        &self,
        chunk: &C,
    ) -> Result<(), crate::error::Error>
    where
        C: Serialize,
    {
        let Some(response_id) = self.primary_id.as_deref() else {
            return Ok(());
        };
        let created_at_seed = now_secs() as i64;
        insert_response_blob(
            &self.pool,
            self.tier,
            response_id,
            chunk,
            created_at_seed,
        )
        .await?;
        Ok(())
    }
}

/// Listener loop. One iteration:
///
/// 1. Block on `rx.recv()` for the first chunk of a batch.
/// 2. Drain any other chunks queued behind it via `try_recv`,
///    `.push()`-aggregating them into the first.
/// 3. Apply the aggregated chunk to the state.
/// 4. If this was the first successful batch, flip `written_tx` to
///    `true` (powers `LogWriter::wait_written_once`).
/// 5. If `primary_id` just became known, fire the ready oneshot.
///
/// On `recv() = None` (sender dropped via `finalize`), the loop
/// exits cleanly. On any DB error from `apply_chunk`, the loop
/// exits with `Err`; subsequent sender sends fail with `SendError`,
/// which `LogWriter::write` maps to a stable `Error::Instance`.
async fn listener_loop<C>(
    mut rx: mpsc::UnboundedReceiver<C>,
    mut state: LogWriterState<C>,
    ready_tx: oneshot::Sender<String>,
    written_tx: watch::Sender<bool>,
) -> Result<(), crate::error::Error>
where
    C: WriterChunk
        + AgentCompletionIds
        + ChunkAgentRefs
        + ChunkPush
        + Clone
        + Serialize
        + Send
        + Sync,
{
    let mut ready_tx = Some(ready_tx);
    let mut written_fired = false;
    // Cumulative aggregate of every chunk across the whole stream. Each
    // iteration's `agg` is only a partial slice (the wire is per-message
    // deltas); folding every batch in here builds the complete response
    // that is written as the response blob exactly once, after the last
    // chunk. Without this the blob would be overwritten with whatever
    // partial batch arrived last, dropping earlier tool-calls.
    let mut accumulated: Option<C> = None;
    while let Some(first) = rx.recv().await {
        // Fold `first` into the stream-wide aggregate, then drain any
        // chunks queued behind it into the same aggregate. `accumulated`
        // is the cumulative roll-up of every chunk seen so far — NOT a
        // per-batch slice. Draining the queue only collapses how OFTEN
        // the persistence pass runs; what it persists from is always
        // the full accumulator.
        // Definition sources ride the RAW chunks (agent_inline is
        // first-chunk-only), so scan each incoming chunk before it
        // dissolves into the aggregate.
        let mut agent_refs: Vec<(String, crate::db::agent_refs::AgentRefValue)> =
            Vec::new();
        first.collect_agent_refs(&mut agent_refs);
        if let Some(acc) = accumulated.as_mut() {
            acc.push(&first);
        } else {
            accumulated = Some(first.clone());
        }
        while let Ok(next) = rx.try_recv() {
            next.collect_agent_refs(&mut agent_refs);
            if let Some(acc) = accumulated.as_mut() {
                acc.push(&next);
            }
        }
        let acc = accumulated
            .as_ref()
            .expect("accumulated is Some: set or pushed above");

        // On the very first chunk: learn the response_id and write the
        // request blob once, before any content row references it.
        if state.primary_id.is_none() {
            let response_id = acc.primary_id().to_string();
            state.write_request_blob(&response_id).await?;
            // Agent tier: unpack the REQUEST into request_message rows
            // (written first) and register the agent_ref from the
            // request's agent field — never the response chunk.
            if let Some(aih) = acc.agent_tier_aih() {
                state.write_agent_request_messages(&response_id, aih).await?;
                if let Some(value) = state.request_agent_ref.take() {
                    crate::db::agent_refs::upsert(&state.pool, aih, value)
                        .await?;
                }
            }
            state.primary_id = Some(response_id);
        }

        // Persist rows from the cumulative aggregate, never a per-batch
        // slice. A tool call's id/name/arguments — and streamed content
        // text — arrive as deltas spread across multiple wire chunks;
        // under load those deltas land in different drain batches. Row
        // generation (`rows.rs`) drops any tool call missing id/name/
        // args, so a per-batch slice that lacked the id/name delta would
        // omit the row entirely (and would overwrite content text with
        // the latest fragment rather than the full run). Walking the
        // full accumulator each pass emits every row from its COMPLETE
        // body; the shadow makes the repeated walk cheap — unchanged
        // rows Skip with zero writes, only genuinely-changed bodies hit
        // the DB.
        state.apply_chunk(acc).await?;

        // Blind agent_refs upserts for every scanned definition
        // source — last write wins by design.
        for (hier, value) in agent_refs {
            crate::db::agent_refs::upsert(&state.pool, &hier, value).await?;
        }

        // First successful apply: flip the watch true exactly once.
        // Subsequent batches don't touch it (the value is already
        // true; no point waking waiters again).
        if !written_fired {
            let _ = written_tx.send(true);
            written_fired = true;
        }
        // Fire the oneshot the first time primary_id becomes known
        // (set above on the first chunk).
        if let Some(tx) = ready_tx.take() {
            match state.primary_id.as_deref() {
                Some(id) => {
                    let _ = tx.send(id.to_string());
                }
                None => {
                    ready_tx = Some(tx);
                }
            }
        }
    }
    // EOF (sender dropped via finalize): every queued chunk is drained.
    if let Some(acc) = accumulated {
        // Mid-stream failure sweep: the caller's stream error (set via
        // `finalize_with_stream_error` before EOF) — or the execution's
        // own TOP-LEVEL in-band error — is logged for every nested
        // agent completion that neither finished (no usage yet) nor
        // carries its own in-band error (already persisted). Runs
        // strictly post-drain, so error rows land AFTER every
        // conversation row.
        let sweep_error = state.stream_error.lock().await.take().or_else(|| {
            (state.chunk_error_fn)(&acc).map(|e| {
                serde_json::to_value(e).unwrap_or_else(|_| e.to_string().into())
            })
        });
        if let Some(value) = sweep_error {
            let ts = now_secs() as i64;
            for status in (state.statuses_fn)(&acc) {
                if status.finished || status.errored {
                    continue;
                }
                let key = (
                    status.agent_instance_hierarchy.to_string(),
                    status.response_id.to_string(),
                );
                if state.logged_errors.contains(&key) {
                    continue;
                }
                // Persist first, then tee — same order as everywhere.
                super::errors::insert_error(
                    &state.pool,
                    status.agent_instance_hierarchy,
                    Some(status.response_id),
                    &value,
                    ts,
                )
                .await?;
                if let Some(tee) = &state.tee {
                    tee.send(super::tee::error_frame(
                        key.0.clone(),
                        Some(key.1.clone()),
                        value.clone(),
                        ts,
                    ));
                }
                state.logged_errors.insert(key);
            }
        }
        // Write the complete response blob exactly once from the
        // cumulative aggregate. (Both skipped when no chunk ever
        // arrived — `accumulated` still `None`.)
        state.write_response_blob(&acc).await?;
    }
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn spawn_writer<C>(
    pool: Pool,
    tier: Tier,
    request_body: serde_json::Value,
    sender_agent_instance_hierarchy: String,
    items_fn: for<'a> fn(&'a C) -> WriterItems<'a>,
    request_messages: Option<
        Vec<objectiveai_sdk::agent::completions::message::Message>,
    >,
    request_agent_ref: Option<crate::db::agent_refs::AgentRefValue>,
    tee: Option<super::tee::ConversationTee>,
    statuses_fn: for<'a> fn(&'a C) -> super::rows::CompletionStatuses<'a>,
    chunk_error_fn: for<'a> fn(&'a C) -> Option<&'a objectiveai_sdk::error::ResponseError>,
) -> (LogWriter<C>, oneshot::Receiver<String>)
where
    C: WriterChunk
        + AgentCompletionIds
        + ChunkAgentRefs
        + ChunkPush
        + Clone
        + Serialize
        + Send
        + Sync
        + 'static,
{
    let (tx, rx) = mpsc::unbounded_channel();
    let (ready_tx, ready_rx) = oneshot::channel();
    let (written_tx, written_rx) = watch::channel(false);
    let stream_error: Arc<Mutex<Option<serde_json::Value>>> =
        Arc::new(Mutex::new(None));
    let state = LogWriterState::new(
        pool,
        tier,
        request_body,
        sender_agent_instance_hierarchy,
        items_fn,
        request_messages,
        request_agent_ref,
        tee,
        statuses_fn,
        chunk_error_fn,
        stream_error.clone(),
    );
    let handle = tokio::spawn(listener_loop(rx, state, ready_tx, written_tx));
    (
        LogWriter {
            tx,
            handle,
            written_rx,
            stream_error,
            _chunk: PhantomData,
        },
        ready_rx,
    )
}

pub fn write_agent_completion(
    pool: &Pool,
    params: &AgentCompletionCreateParams,
    sender_agent_instance_hierarchy: String,
    tee: Option<super::tee::ConversationTee>,
) -> Result<
    (LogWriter<AgentCompletionChunk>, oneshot::Receiver<String>),
    crate::error::Error,
> {
    let body = serde_json::to_value(params)?;
    // Agent tier derives request_message rows + the agent_ref from the
    // REQUEST (not the response chunk). Capture both at construction.
    let request_messages = Some(params.messages.clone());
    let request_agent_ref = match &params.agent {
        objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(base) => {
            crate::db::agent_refs::AgentRefValue::inline(base)
        }
        objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(remote) => {
            crate::db::agent_refs::AgentRefValue::remote(remote)
        }
    };
    Ok(spawn_writer(
        pool.clone(),
        Tier::Agent,
        body,
        sender_agent_instance_hierarchy,
        agent_completion_chunk_rows,
        request_messages,
        request_agent_ref,
        tee,
        super::rows::agent_completion_statuses,
        // Agent tier: spawn's `note_error` owns stream errors; the
        // chunk's own in-band error is logged via `WriterItem::Error`.
        |_| None,
    ))
}

pub fn write_vector_completion(
    pool: &Pool,
    params: &VectorCompletionCreateParams,
    sender_agent_instance_hierarchy: String,
    tee: Option<super::tee::ConversationTee>,
) -> Result<
    (LogWriter<VectorCompletionChunk>, oneshot::Receiver<String>),
    crate::error::Error,
> {
    let body = serde_json::to_value(params)?;
    Ok(spawn_writer(
        pool.clone(),
        Tier::Vector,
        body,
        sender_agent_instance_hierarchy,
        vector_completion_chunk_rows,
        None,
        None,
        tee,
        super::rows::vector_completion_statuses,
        // VectorCompletionChunk carries no top-level error field.
        |_| None,
    ))
}

pub fn write_function_execution(
    pool: &Pool,
    params: &FunctionExecutionCreateParams,
    sender_agent_instance_hierarchy: String,
    tee: Option<super::tee::ConversationTee>,
) -> Result<
    (LogWriter<FunctionExecutionChunk>, oneshot::Receiver<String>),
    crate::error::Error,
> {
    let body = serde_json::to_value(params)?;
    Ok(spawn_writer(
        pool.clone(),
        Tier::Function,
        body,
        sender_agent_instance_hierarchy,
        function_execution_chunk_rows,
        None,
        None,
        tee,
        super::rows::function_execution_statuses,
        |chunk| chunk.error.as_ref(),
    ))
}
