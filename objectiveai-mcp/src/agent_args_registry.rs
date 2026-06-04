//! Per-rmcp-session in-memory map of
//! [`objectiveai_sdk::cli::command::AgentArguments`]. Populated by
//! [`crate::header_session_manager::HeaderSessionManager`] on every
//! `initialize` (fresh, no-session-id POST, or lazy-rehydration of
//! a reconnect with an id we haven't seen this process lifetime),
//! consumed by every tool handler before dispatching to the
//! executor.
//!
//! FULL-REPLACE semantics on every record: a reconnect with a new
//! header set wholesale replaces the prior entry, and missing
//! headers become `None` on the new struct (effectively cleared).
//! Mirrors the lifecycle of
//! `objectiveai-mcp-proxy::Session::transient_headers` and the
//! `SessionRegistry` in `psychological-operations-x-api-mcp`.
//!
//! In-memory only. A process restart silently flushes the map; the
//! CLI re-sends the six `X-OBJECTIVEAI-*` headers on its next
//! request, and the lazy-rehydration path re-captures them.

use std::collections::HashMap;
use std::sync::Arc;

use objectiveai_sdk::cli::command::AgentArguments;
use rmcp::transport::common::server_side_http::SessionId;
use tokio::sync::RwLock;

/// Shared registry of per-session [`AgentArguments`]. Cheap to
/// clone (the inner state is `Arc`'d).
#[derive(Default, Debug, Clone)]
pub struct AgentArgumentsRegistry {
    inner: Arc<RwLock<HashMap<SessionId, Arc<AgentArguments>>>>,
}

impl AgentArgumentsRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or FULL-REPLACE the entry for `id` with `args`. Any
    /// prior bag (with whatever fields it had) is discarded.
    pub async fn record(&self, id: SessionId, args: Arc<AgentArguments>) {
        self.inner.write().await.insert(id, args);
    }

    /// Look up the current bag for `id`. Returns `None` if no
    /// session has been registered under this id.
    pub async fn get(&self, id: &SessionId) -> Option<Arc<AgentArguments>> {
        self.inner.read().await.get(id).cloned()
    }

    /// Drop the entry for `id`. Returns the prior value if any.
    pub async fn remove(&self, id: &SessionId) -> Option<Arc<AgentArguments>> {
        self.inner.write().await.remove(id)
    }
}
