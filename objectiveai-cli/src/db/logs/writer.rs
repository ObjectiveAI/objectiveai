//! `LogWriter<C>` — postgres log writer driven by the chunk-rows
//! iterator family.
//!
//! Lifecycle:
//!
//! 1. **Construction** via one of the three tier factories
//!    ([`write_agent_completion`] / [`write_vector_completion`] /
//!    [`write_function_execution`]). The request body is serialized
//!    to JSONB once up-front and stashed; the matching `*_chunk_rows`
//!    walker is wired in as a function pointer.
//! 2. **First chunk**: capture the chunk's `response_id`, INSERT the
//!    request blob (no-op on re-run thanks to shadow gating), INSERT
//!    the response blob.
//! 3. **Every chunk**: walk `chunk_rows(chunk)`. For each yielded
//!    [`RowValue`], the shadow returns Insert / Update / Skip; the
//!    writer dispatches the matching flat SQL (no ON CONFLICT). After
//!    the per-row walk, the response blob is UPDATEd if its body
//!    fingerprint changed.
//! 4. **Finalize**: nothing extra — the per-chunk flow already
//!    persisted the latest state.
//!
//! The writer is the sole caller of [`super::write`] and is
//! single-instance per stream, so the shadow's verdict is the
//! authoritative ordering signal — no DB-side conflict logic needed.

use std::marker::PhantomData;

use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::completions::response::streaming::{
    AgentCompletionChunk, AgentCompletionIds,
};
use objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
use objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams;
use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;
use serde::Serialize;

use crate::db::Pool;
use crate::db::messages::Queue;
use crate::db::pending::PendingNotification;

use super::row::{RowTable, RowsIter};
use super::rows::{
    agent_completion_chunk_rows, function_execution_chunk_rows, vector_completion_chunk_rows,
};
use super::shadow::{Shadow, WriteOp, blob_fingerprint};
use super::write::{
    Tier, insert_request_blob, insert_response_blob, update_response_blob, write_value,
};

/// Per-chunk-type metadata the writer needs at runtime. Provides the
/// primary response id (PK of the tier blob tables) and the optional
/// `agent_instance_hierarchy` column value (only present for the
/// agent tier).
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
    /// Pre-serialized request body; written verbatim on first chunk.
    request_body: serde_json::Value,
    /// `chunk_rows` walker for `C`. Static function pointer; cheap to
    /// store, no allocation per chunk.
    rows_fn: for<'a> fn(&'a C) -> RowsIter<'a>,
    primary_id: Option<String>,
    shadow: Shadow,
    queue: Option<Queue>,
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
            queue: None,
            caller_agent_instance_hierarchy: None,
            _chunk: PhantomData,
        }
    }

    pub fn with_caller_agent_instance_hierarchy(mut self, caller: Option<String>) -> Self {
        self.caller_agent_instance_hierarchy = caller;
        self
    }

    pub fn with_queue(mut self, queue: Queue) -> Self {
        self.queue = Some(queue);
        self
    }

    pub fn queue(&self) -> Option<&Queue> {
        self.queue.as_ref()
    }

    pub fn primary_id(&self) -> Option<&str> {
        self.primary_id.as_deref()
    }

    /// Reserve a per-agent notification index for the caller to queue.
    /// Notification content extraction will land via the same row
    /// iterator path once the request-side message tables come online
    /// — for now the writer just reserves the index.
    pub async fn write_notification(
        &mut self,
        agent_instance_hierarchy: &str,
        response_id: &str,
        _content: &RichContent,
    ) -> Result<PendingNotification, crate::error::Error> {
        match &self.queue {
            Some(q) => Ok(q
                .reserve_pending_notification(agent_instance_hierarchy, response_id)
                .await?),
            None => Ok(PendingNotification {
                agent_instance_hierarchy: agent_instance_hierarchy.to_string(),
                response_id: response_id.to_string(),
                index: 0,
                path: String::new(),
                timestamp: now_secs(),
            }),
        }
    }

    pub async fn write(
        &mut self,
        chunk: &C,
        _pending: &mut Vec<PendingNotification>,
    ) -> Result<Vec<(String, RequestMessageKind)>, crate::error::Error>
    where
        C: WriterChunk + AgentCompletionIds + Serialize + Clone + Send + Sync,
    {
        // First-chunk path: stamp the primary id, write the request
        // blob (once), seed the response blob.
        let first_chunk = self.primary_id.is_none();
        let response_id = if let Some(id) = self.primary_id.as_ref() {
            id.clone()
        } else {
            let id = chunk.primary_id().to_string();
            self.primary_id = Some(id.clone());
            id
        };
        let agent_hierarchy = chunk.agent_instance_hierarchy_opt();
        let created_at_seed = now_secs() as i64;

        if first_chunk {
            // Tier request blob. Written once; the shadow gate ensures
            // a retry doesn't re-INSERT.
            let req_body_bytes = serde_json::to_vec(&self.request_body)?;
            if self
                .shadow
                .record_blob(request_table(self.tier), &response_id, &req_body_bytes)
                != WriteOp::Skip
            {
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
            }
        }

        // Streaming-content rows: walk, ask the shadow, dispatch flat
        // INSERT/UPDATE for non-Skip verdicts.
        for value in (self.rows_fn)(chunk) {
            let op = self.shadow.record(&value);
            if op != WriteOp::Skip {
                write_value(&self.pool, op, value).await?;
            }
        }

        // Tier response blob. Diff by body bytes; first chunk inserts,
        // later chunks update on change, skip on byte-identical body.
        let response_bytes = serde_json::to_vec(chunk)?;
        let blob_op =
            self.shadow
                .record_blob(response_table(self.tier), &response_id, &response_bytes);
        match blob_op {
            WriteOp::Insert => {
                insert_response_blob(
                    &self.pool,
                    self.tier,
                    &response_id,
                    agent_hierarchy,
                    chunk,
                    created_at_seed,
                )
                .await?;
            }
            WriteOp::Update => {
                update_response_blob(&self.pool, self.tier, &response_id, chunk, created_at_seed)
                    .await?;
            }
            WriteOp::Skip => {}
        }

        let _ = blob_fingerprint(&response_bytes);
        Ok(Vec::new())
    }

    /// Stream-over hook. Per-chunk writes already persisted the latest
    /// state; nothing to do here today.
    pub async fn finalize(
        &mut self,
        _pending: &mut Vec<PendingNotification>,
    ) -> Result<Vec<(String, RequestMessageKind)>, crate::error::Error>
    where
        C: AgentCompletionIds,
    {
        Ok(Vec::new())
    }
}

fn request_table(tier: Tier) -> RowTable {
    match tier {
        Tier::Agent => RowTable::AgentCompletionRequests,
        Tier::Vector => RowTable::VectorCompletionRequests,
        Tier::Function => RowTable::FunctionExecutionRequests,
    }
}

fn response_table(tier: Tier) -> RowTable {
    match tier {
        Tier::Agent => RowTable::AgentCompletionResponses,
        Tier::Vector => RowTable::VectorCompletionResponses,
        Tier::Function => RowTable::FunctionExecutionResponses,
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// =====================================================================
// Factory entry points
// =====================================================================

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
