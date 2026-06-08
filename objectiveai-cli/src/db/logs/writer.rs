//! `LogWriter<C>` — postgres log writer driven by the chunk-rows
//! iterator family.
//!
//! Per-chunk lifecycle:
//!
//! 1. **First chunk**: capture the chunk's `response_id`, INSERT the
//!    request blob (no `agent_instance_hierarchy` on the blob — that
//!    linkage lives in `logs.messages`).
//! 2. **Every chunk**: walk `chunk_rows(chunk)`, gate each yielded
//!    [`RowValue`] through the shadow (Skip path is pure-memory),
//!    bucket the survivors by `agent_instance_hierarchy`. For every
//!    agent the writer hasn't seen yet in this stream's lifetime,
//!    prepend a `logs.messages` row that registers the request blob
//!    in that agent's history — this is always the first item in the
//!    bucket, so postgres's BIGSERIAL gives it an `"index"` strictly
//!    less than the streaming-content rows that follow.
//! 3. **Per-bucket execution**: rows within one agent's bucket fire
//!    sequentially (so the per-agent ORDER BY `"index"` matches the
//!    iterator's order). All buckets fire concurrently via
//!    `try_join_all`. The response blob update runs in parallel with
//!    the bucket fan-out via `tokio::join!`.

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

use crate::db::Pool;

use super::row::{RowValue, RowsIter};
use super::rows::{
    agent_completion_chunk_rows, function_execution_chunk_rows, vector_completion_chunk_rows,
};
use super::shadow::{Shadow, WriteOp};
use super::write::{
    Tier, insert_request_blob, insert_request_messages_row, insert_response_blob,
    update_response_blob, write_value,
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

pub struct LogWriter<C> {
    pool: Pool,
    tier: Tier,
    request_body: serde_json::Value,
    rows_fn: for<'a> fn(&'a C) -> RowsIter<'a>,
    primary_id: Option<String>,
    /// Per-streaming-content-row shadow. Skip path is allocation-free.
    shadow: Shadow,
    /// Last-written response blob bytes — `PartialEq` against the
    /// next tick's serialized chunk decides Insert / Update / Skip.
    last_response_blob: Option<Vec<u8>>,
    /// Once-flag for the request blob INSERT.
    request_written: bool,
    /// Every `agent_instance_hierarchy` we've observed in this
    /// stream's lifetime. The first time an agent appears in the row
    /// iterator we insert a `logs.messages` row registering the
    /// request blob in that agent's history; subsequent ticks see
    /// the agent already-marked and skip the registration.
    seen_agents: HashSet<String>,
    _chunk: PhantomData<fn() -> C>,
}

impl<C> LogWriter<C> {
    fn new(
        pool: Pool,
        tier: Tier,
        request_body: serde_json::Value,
        rows_fn: for<'a> fn(&'a C) -> RowsIter<'a>,
    ) -> Self {
        Self {
            pool,
            tier,
            request_body,
            rows_fn,
            primary_id: None,
            shadow: Shadow::new(),
            last_response_blob: None,
            request_written: false,
            seen_agents: HashSet::new(),
            _chunk: PhantomData,
        }
    }

    pub fn primary_id(&self) -> Option<&str> {
        self.primary_id.as_deref()
    }

    pub async fn write(
        &mut self,
        chunk: &C,
    ) -> Result<(), crate::error::Error>
    where
        C: WriterChunk + AgentCompletionIds + Serialize + Clone + Send + Sync,
    {
        // First chunk: stamp the primary id, write the request blob
        // ONCE. The request blob has no agent_instance_hierarchy.
        if self.primary_id.is_none() {
            self.primary_id = Some(chunk.primary_id().to_string());
        }
        let response_id = self.primary_id.clone().expect("set above");
        let created_at_seed = now_secs() as i64;

        if !self.request_written {
            let request_body = self.request_body.clone();
            insert_request_blob(
                &self.pool,
                self.tier,
                &response_id,
                &request_body,
                created_at_seed,
            )
            .await?;
            self.request_written = true;
        }

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

        // Serialize the response chunk once, diff against the
        // last-written blob.
        let response_bytes = serde_json::to_vec(chunk)?;
        let blob_op = match &self.last_response_blob {
            Some(prev) if prev == &response_bytes => WriteOp::Skip,
            Some(_) => WriteOp::Update,
            None => WriteOp::Insert,
        };

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

        let content_fut = futures::future::try_join_all(bucket_futures);
        let blob_fut = async {
            match blob_op {
                WriteOp::Insert => {
                    insert_response_blob(
                        pool,
                        tier,
                        resp_id,
                        chunk,
                        created_at_seed,
                    )
                    .await
                }
                WriteOp::Update => {
                    update_response_blob(pool, tier, resp_id, chunk, created_at_seed).await
                }
                WriteOp::Skip => Ok(()),
            }
        };
        let (content_res, blob_res) = tokio::join!(content_fut, blob_fut);
        content_res?;
        blob_res?;

        if blob_op != WriteOp::Skip {
            self.last_response_blob = Some(response_bytes);
        }

        Ok(())
    }

    /// Stream-over hook. The per-chunk writes have already persisted
    /// the latest state; nothing to flush here today.
    pub async fn finalize(&mut self) -> Result<(), crate::error::Error> {
        Ok(())
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn write_agent_completion(
    pool: &Pool,
    params: &AgentCompletionCreateParams,
) -> Result<LogWriter<AgentCompletionChunk>, crate::error::Error> {
    let body = serde_json::to_value(params)?;
    Ok(LogWriter::new(
        pool.clone(),
        Tier::Agent,
        body,
        agent_completion_chunk_rows,
    ))
}

pub fn write_vector_completion(
    pool: &Pool,
    params: &VectorCompletionCreateParams,
) -> Result<LogWriter<VectorCompletionChunk>, crate::error::Error> {
    let body = serde_json::to_value(params)?;
    Ok(LogWriter::new(
        pool.clone(),
        Tier::Vector,
        body,
        vector_completion_chunk_rows,
    ))
}

pub fn write_function_execution(
    pool: &Pool,
    params: &FunctionExecutionCreateParams,
) -> Result<LogWriter<FunctionExecutionChunk>, crate::error::Error> {
    let body = serde_json::to_value(params)?;
    Ok(LogWriter::new(
        pool.clone(),
        Tier::Function,
        body,
        function_execution_chunk_rows,
    ))
}
