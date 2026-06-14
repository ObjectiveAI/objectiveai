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
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::db::Pool;

use super::row::{RowValue, RowsIter};
use super::rows::{
    agent_completion_chunk_rows, function_execution_chunk_rows, vector_completion_chunk_rows,
};
use super::shadow::{Shadow, WriteOp};
use super::write::{
    Tier, insert_request_blob, insert_request_messages_row, insert_response_blob,
    write_value,
};

pub trait WriterChunk {
    fn primary_id(&self) -> &str;
}

impl WriterChunk for AgentCompletionChunk {
    fn primary_id(&self) -> &str {
        self.id.as_str()
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
}

/// All the per-stream state the listener task owns. Was previously
/// inlined onto `LogWriter`; now lives entirely inside the spawned
/// task so the handle stays send-and-clone-cheap.
struct LogWriterState<C> {
    pool: Pool,
    tier: Tier,
    request_body: serde_json::Value,
    /// AIH of the caller who issued the request that spawned this
    /// writer (pulled from `ctx.config.agent_instance_hierarchy` at
    /// `spawn_writer` time). Written into the request blob row at
    /// `insert_request_blob` time. Constant for the writer's
    /// lifetime — one request = one sender.
    sender_agent_instance_hierarchy: String,
    rows_fn: for<'a> fn(&'a C) -> RowsIter<'a>,
    primary_id: Option<String>,
    /// Per-streaming-content-row shadow. Skip path is allocation-free.
    shadow: Shadow,
    /// Every `agent_instance_hierarchy` we've observed in this
    /// stream's lifetime. The first time an agent appears in the row
    /// iterator we insert a `objectiveai.messages` row registering the
    /// request blob in that agent's history; subsequent ticks see
    /// the agent already-marked and skip the registration.
    seen_agents: HashSet<String>,
    _chunk: PhantomData<fn() -> C>,
}

impl<C> LogWriterState<C> {
    fn new(
        pool: Pool,
        tier: Tier,
        request_body: serde_json::Value,
        sender_agent_instance_hierarchy: String,
        rows_fn: for<'a> fn(&'a C) -> RowsIter<'a>,
    ) -> Self {
        Self {
            pool,
            tier,
            request_body,
            sender_agent_instance_hierarchy,
            rows_fn,
            primary_id: None,
            shadow: Shadow::new(),
            seen_agents: HashSet::new(),
            _chunk: PhantomData,
        }
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

        // Walk rows, gate via shadow, bucket survivors by
        // agent_instance_hierarchy. Vec inside the HashMap preserves
        // iterator order so per-bucket sequential awaits match
        // chunk_rows()'s ordering.
        let mut buckets: HashMap<&str, Vec<(WriteOp, RowValue<'_>)>> = HashMap::new();
        for value in (self.rows_fn)(chunk) {
            let key = value.agent_instance_hierarchy();
            match self.shadow.record(&value) {
                WriteOp::Skip => continue,
                op => buckets.entry(key).or_default().push((op, value)),
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
    C: WriterChunk + AgentCompletionIds + ChunkPush + Clone + Serialize + Send + Sync,
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
        if let Some(acc) = accumulated.as_mut() {
            acc.push(&first);
        } else {
            accumulated = Some(first.clone());
        }
        while let Ok(next) = rx.try_recv() {
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
    // EOF (sender dropped via finalize): write the complete response
    // blob exactly once from the cumulative aggregate. Skipped when no
    // chunk ever arrived (primary_id still unset).
    if let Some(acc) = accumulated {
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
    rows_fn: for<'a> fn(&'a C) -> RowsIter<'a>,
) -> (LogWriter<C>, oneshot::Receiver<String>)
where
    C: WriterChunk + AgentCompletionIds + ChunkPush + Clone + Serialize + Send + Sync + 'static,
{
    let (tx, rx) = mpsc::unbounded_channel();
    let (ready_tx, ready_rx) = oneshot::channel();
    let (written_tx, written_rx) = watch::channel(false);
    let state = LogWriterState::new(
        pool,
        tier,
        request_body,
        sender_agent_instance_hierarchy,
        rows_fn,
    );
    let handle = tokio::spawn(listener_loop(rx, state, ready_tx, written_tx));
    (
        LogWriter {
            tx,
            handle,
            written_rx,
            _chunk: PhantomData,
        },
        ready_rx,
    )
}

pub fn write_agent_completion(
    pool: &Pool,
    params: &AgentCompletionCreateParams,
    sender_agent_instance_hierarchy: String,
) -> Result<
    (LogWriter<AgentCompletionChunk>, oneshot::Receiver<String>),
    crate::error::Error,
> {
    let body = serde_json::to_value(params)?;
    Ok(spawn_writer(
        pool.clone(),
        Tier::Agent,
        body,
        sender_agent_instance_hierarchy,
        agent_completion_chunk_rows,
    ))
}

pub fn write_vector_completion(
    pool: &Pool,
    params: &VectorCompletionCreateParams,
    sender_agent_instance_hierarchy: String,
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
    ))
}

pub fn write_function_execution(
    pool: &Pool,
    params: &FunctionExecutionCreateParams,
    sender_agent_instance_hierarchy: String,
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
    ))
}
