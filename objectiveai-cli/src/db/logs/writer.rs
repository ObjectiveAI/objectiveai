//! `LogWriter<C>` — postgres log writer driven by the chunk-rows
//! iterator family.
//!
//! Lifecycle:
//!
//! 1. **Construction** via one of the three tier factories
//!    ([`write_agent_completion`] / [`write_vector_completion`] /
//!    [`write_function_execution`]). The request body is serialized
//!    to JSONB up-front and stashed; the matching `*_chunk_rows`
//!    walker is wired in as a function pointer.
//! 2. **First chunk**: capture the chunk's `response_id`, INSERT the
//!    request blob once, INSERT the response blob.
//! 3. **Every chunk**: walk `chunk_rows(chunk)`. For each yielded
//!    [`RowValue`], the shadow returns
//!    [`Insert`](crate::db::logs::WriteOp::Insert) /
//!    [`Update`](crate::db::logs::WriteOp::Update) /
//!    [`Skip`](crate::db::logs::WriteOp::Skip). The Skip path is
//!    pure-memory: borrowed-key probe + `PartialEq` on the stored
//!    body, no allocation. After the per-row walk, the response blob
//!    is UPDATEd if its bytes changed.

use std::marker::PhantomData;

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
    Tier, insert_request_blob, insert_response_blob, update_response_blob, write_value,
};

pub trait WriterChunk {
    fn primary_id(&self) -> &str;
    fn agent_instance_hierarchy_opt(&self) -> Option<&str>;
}

impl WriterChunk for AgentCompletionChunk {
    fn primary_id(&self) -> &str {
        self.id.as_str()
    }
    fn agent_instance_hierarchy_opt(&self) -> Option<&str> {
        Some(self.agent_instance_hierarchy.as_str())
    }
}
impl WriterChunk for VectorCompletionChunk {
    fn primary_id(&self) -> &str {
        self.id.as_str()
    }
    fn agent_instance_hierarchy_opt(&self) -> Option<&str> {
        None
    }
}
impl WriterChunk for FunctionExecutionChunk {
    fn primary_id(&self) -> &str {
        self.id.as_str()
    }
    fn agent_instance_hierarchy_opt(&self) -> Option<&str> {
        None
    }
}

pub struct LogWriter<C> {
    pool: Pool,
    tier: Tier,
    request_body: serde_json::Value,
    rows_fn: for<'a> fn(&'a C) -> RowsIter<'a>,
    primary_id: Option<String>,
    /// Per-streaming-content-row shadow: hashed by borrowed key, body
    /// compared via [`PartialEq`]. See [`Shadow`] for the
    /// allocation-free Skip path.
    shadow: Shadow,
    /// Last-written response blob bytes for diff detection. `None`
    /// before the first chunk; `Some(bytes)` after at least one
    /// successful response-blob write. PartialEq on `Vec<u8>` bails
    /// fast on length / byte mismatch.
    last_response_blob: Option<Vec<u8>>,
    /// Once-flag for the request blob INSERT.
    request_written: bool,
    #[allow(dead_code)]
    caller_agent_instance_hierarchy: Option<String>,
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
            caller_agent_instance_hierarchy: None,
            _chunk: PhantomData,
        }
    }

    pub fn with_caller_agent_instance_hierarchy(mut self, caller: Option<String>) -> Self {
        self.caller_agent_instance_hierarchy = caller;
        self
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
        // First-chunk path: stamp the primary id, write the request
        // blob once.
        if self.primary_id.is_none() {
            self.primary_id = Some(chunk.primary_id().to_string());
        }
        let response_id = self.primary_id.clone().expect("set above");
        let agent_hierarchy = chunk.agent_instance_hierarchy_opt();
        let created_at_seed = now_secs() as i64;

        if !self.request_written {
            let request_body = self.request_body.clone();
            insert_request_blob(
                &self.pool,
                self.tier,
                &response_id,
                agent_hierarchy,
                &request_body,
                created_at_seed,
            )
            .await?;
            self.request_written = true;
        }

        // Stage 1: walk streaming-content rows, gate each through the
        // shadow (borrowed-key probe + PartialEq body diff). Collect
        // the survivors — the Skip path stays pure-memory; only
        // non-Skip rows make it into `dispatched`. The collected vec
        // borrows from `chunk`, which outlives this function, so the
        // futures below can reference the borrowed payloads directly.
        let dispatched: Vec<(WriteOp, RowValue<'_>)> = (self.rows_fn)(chunk)
            .filter_map(|value| match self.shadow.record(&value) {
                WriteOp::Skip => None,
                op => Some((op, value)),
            })
            .collect();

        // Stage 2: serialize the response chunk once (we need the
        // bytes anyway), diff against the last-written blob.
        let response_bytes = serde_json::to_vec(chunk)?;
        let blob_op = match &self.last_response_blob {
            Some(prev) if prev == &response_bytes => WriteOp::Skip,
            Some(_) => WriteOp::Update,
            None => WriteOp::Insert,
        };

        // Stage 3: fan out every surviving streaming-content write
        // AND the response-blob write concurrently. All futures share
        // the live `chunk`'s borrow scope, so we never need to clone
        // payloads into the futures.
        let pool = &self.pool;
        let content_fut = futures::future::try_join_all(
            dispatched
                .iter()
                .map(|(op, value)| write_value(pool, *op, value)),
        );
        let blob_fut = async {
            match blob_op {
                WriteOp::Insert => {
                    insert_response_blob(
                        pool,
                        self.tier,
                        &response_id,
                        agent_hierarchy,
                        chunk,
                        created_at_seed,
                    )
                    .await
                }
                WriteOp::Update => {
                    update_response_blob(
                        pool,
                        self.tier,
                        &response_id,
                        chunk,
                        created_at_seed,
                    )
                    .await
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
