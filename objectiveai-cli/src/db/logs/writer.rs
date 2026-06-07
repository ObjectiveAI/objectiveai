//! `LogWriter<C>` — scaffolding for the postgres log writer.
//!
//! Mirrors the API the streaming bridge expects (`write` / `finalize`
//! / `primary_id` / `with_caller_agent_instance_hierarchy` /
//! `write_notification`) so the surrounding cli compiles. The actual
//! strip-and-insert bodies are stubbed pending the content-table
//! schema landing in `db::init`.

use std::marker::PhantomData;

use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds;
use objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind;

use crate::db::Pool;
use crate::db::messages::Queue;
use crate::db::pending::PendingNotification;

/// Streaming chunk → postgres log rows. `C` is the wire chunk type
/// (`AgentCompletionChunk` / `VectorCompletionChunk` /
/// `FunctionExecutionChunk`). Construct via the per-tier factory
/// functions in this module ([`write_agent_completion`] etc.).
pub struct LogWriter<C> {
    #[allow(dead_code)]
    pool: Pool,
    /// Per-stream caller lineage prefix prepended to every chunk-emitted
    /// `agent_instance_hierarchy` before it lands in
    /// `messages.agent_instance_hierarchy`. `None` keeps the raw chunk
    /// value.
    #[allow(dead_code)]
    caller_agent_instance_hierarchy: Option<String>,
    /// Shared per-agent-id messages-table handle. `None` disables
    /// per-agent message persistence entirely.
    queue: Option<Queue>,
    /// Captured primary chunk id (the id of the first written row).
    /// Available once at least one chunk has been processed.
    primary_id: Option<String>,
    _chunk: PhantomData<fn() -> C>,
}

impl<C> LogWriter<C> {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            caller_agent_instance_hierarchy: None,
            queue: None,
            primary_id: None,
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

    /// Reserve the agent's next db index and persist the notification's
    /// content. Stub: returns a bare `PendingNotification` without
    /// touching the content tables.
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

    /// Process one incoming chunk. Stub: captures the primary id from
    /// the first id-bearing chunk and returns an empty inserted-row
    /// list. The strip + ON CONFLICT insert sequence comes next.
    pub async fn write(
        &mut self,
        chunk: &C,
        _pending: &mut Vec<PendingNotification>,
    ) -> Result<Vec<(String, RequestMessageKind)>, crate::error::Error>
    where
        C: AgentCompletionIds + Clone,
    {
        if self.primary_id.is_none() {
            if let Some(first) = chunk.agent_completion_ids().next() {
                self.primary_id = Some(first.to_string());
            }
        }
        Ok(Vec::new())
    }

    /// Stream-over hook. Stub: returns the empty inserted-row list.
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

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---- Factory entry points -------------------------------------------

use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
use objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams;
use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;

/// Build the writer for an agent-completion stream. Stub: captures
/// the request alongside the pool; the strip+insert sequence lands
/// once the content tables are wired up.
pub fn write_agent_completion(
    pool: &Pool,
    _params: &AgentCompletionCreateParams,
) -> Result<LogWriter<AgentCompletionChunk>, crate::error::Error> {
    Ok(LogWriter::new(pool.clone()))
}

pub fn write_vector_completion(
    pool: &Pool,
    _params: &VectorCompletionCreateParams,
) -> Result<LogWriter<VectorCompletionChunk>, crate::error::Error> {
    Ok(LogWriter::new(pool.clone()))
}

pub fn write_function_execution(
    pool: &Pool,
    _params: &FunctionExecutionCreateParams,
) -> Result<LogWriter<FunctionExecutionChunk>, crate::error::Error> {
    Ok(LogWriter::new(pool.clone()))
}
