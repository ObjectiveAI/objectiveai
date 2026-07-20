//! Per-rmcp-session in-memory map of [`SessionState`]. Populated by
//! [`crate::header_session_manager::HeaderSessionManager`] on every
//! `initialize` (fresh, no-session-id POST, or lazy-rehydration of
//! a reconnect with an id we haven't seen this process lifetime),
//! consumed by every tool handler before dispatching to the
//! executor.
//!
//! FULL-REPLACE semantics on every record: a reconnect with a new
//! header set wholesale replaces the prior entry, and missing
//! headers become `None` (filter values) / `true` (root) on the new
//! struct. Mirrors the lifecycle of
//! `objectiveai-mcp-proxy::Session::transient_headers` and the
//! `SessionRegistry` in `psychological-operations-x-api-mcp`.
//!
//! In-memory only. A process restart silently flushes the map; the
//! CLI re-sends the headers on its next request, and the
//! lazy-rehydration path re-captures them.

use std::collections::HashMap;
use std::sync::Arc;

use objectiveai_sdk::cli::command::AgentArguments;
use rmcp::transport::common::server_side_http::SessionId;
use tokio::sync::RwLock;

/// Per-session state recorded by the header parser. Wraps the
/// legacy [`AgentArguments`] identity bag alongside the three
/// optional `X-OBJECTIVEAI-MCP-*` filter values, so a single
/// `(record, get, remove)` call cycle covers both. The wrapper
/// shape keeps the registry's inner map single — one entry per
/// session, projected by callers via `.args` or the `mcp_*`
/// fields as needed.
///
/// `mcp_root` resolves to `true` when the
/// `X-OBJECTIVEAI-MCP-ROOT` header is absent on connect — the
/// header parser writes the resolved value here, so anything
/// stored is the final per-session decision.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub args: AgentArguments,
    pub mcp_root: bool,
}

/// Shared registry of per-session [`SessionState`]. Cheap to clone
/// (the inner state is `Arc`'d).
#[derive(Default, Debug, Clone)]
pub struct AgentArgumentsRegistry {
    inner: Arc<RwLock<HashMap<SessionId, Arc<SessionState>>>>,
}

impl AgentArgumentsRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or FULL-REPLACE the entry for `id` with `state`. Any
    /// prior state (with whatever fields it had) is discarded.
    pub async fn record(&self, id: SessionId, state: Arc<SessionState>) {
        self.inner.write().await.insert(id, state);
    }

    /// Look up the current state for `id`. Returns `None` if no
    /// session has been registered under this id.
    pub async fn get(&self, id: &SessionId) -> Option<Arc<SessionState>> {
        self.inner.read().await.get(id).cloned()
    }

    /// Drop the entry for `id`. Returns the prior value if any.
    pub async fn remove(&self, id: &SessionId) -> Option<Arc<SessionState>> {
        self.inner.write().await.remove(id)
    }
}
