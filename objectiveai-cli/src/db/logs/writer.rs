//! `LogWriter<C>` — scaffolding for the iterator-driven postgres
//! log writer.
//!
//! Final design (in flight): every chunk type exposes
//! `log_rows(&self) -> impl Iterator<Item = LogValue<'_>>`. The
//! writer pulls one row at a time, looks up its key in a shadow map,
//! and UPSERTs into the matching table only when the row body changed
//! since the last tick. Tier-blob writes (request on first chunk,
//! response on every tick) happen via separate `write_request_blob`
//! / `write_response_blob` calls.
//!
//! Today this stub keeps the streaming-bridge API (`write` /
//! `finalize` / `primary_id` / `with_caller_agent_instance_hierarchy`
//! / `with_queue` / `write_notification`) so the surrounding cli
//! compiles. The iterator hookups land next.

use std::marker::PhantomData;

use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionIds;
use objectiveai_sdk::cli::command::agents::instances::read::subscribe::RequestMessageKind;

use crate::db::Pool;
use crate::db::messages::Queue;
use crate::db::pending::PendingNotification;

pub struct LogWriter<C> {
    #[allow(dead_code)]
    pool: Pool,
    #[allow(dead_code)]
    caller_agent_instance_hierarchy: Option<String>,
    queue: Option<Queue>,
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
        C: AgentCompletionIds + Clone,
    {
        if self.primary_id.is_none() {
            if let Some(first) = chunk.agent_completion_ids().next() {
                self.primary_id = Some(first.to_string());
            }
        }
        Ok(Vec::new())
    }

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

// ---- Factory entry points (stubs) ----

use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
use objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams;
use objectiveai_sdk::functions::executions::response::streaming::FunctionExecutionChunk;
use objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams;
use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;

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
