//! Construction of the daemon's in-process MCP service — the fold-in
//! replacement for the standalone server's `setup` (which bound its
//! own listener and printed a readiness handshake; here the service
//! nests into the daemon's existing router instead).

use std::sync::Arc;

use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use tokio_util::sync::CancellationToken;

use super::agent_args_registry::AgentArgumentsRegistry;
use super::header_session_manager::HeaderSessionManager;
use super::objectiveai::ObjectiveAiMcpCli;
use crate::executor::DaemonCommandExecutor;

/// The concrete rmcp service the daemon mounts at `/mcp`.
pub type McpService = StreamableHttpService<
    ObjectiveAiMcpCli<DaemonCommandExecutor>,
    HeaderSessionManager<DaemonCommandExecutor>,
>;

/// Build the MCP service over the daemon's in-process executor.
///
/// The executor carries the daemon's `GlobalContext` + BASE
/// `ScopedContext`; each tool call derives a fresh per-call scope from
/// the session's `X-OBJECTIVEAI-*` identity headers (the plugin trio
/// is never wire-settable — the executor nulls it). The `/listen`
/// broadcast tee is `None`: MCP-driven runs execute in-process without
/// a per-run listener id, so they are not fanned onto `/listen` (the
/// retired subprocess path re-entered through the CLI and was).
///
/// The returned [`CancellationToken`] owns the service's rmcp session
/// workers; it lives as long as the daemon (dropped only at process
/// end — the daemon never tears its HTTP server down separately).
pub fn service(executor: DaemonCommandExecutor) -> (McpService, CancellationToken) {
    let executor = Arc::new(executor);

    // Shared per-rmcp-session bag of SessionState (the AgentArguments
    // identity bag plus the X-OBJECTIVEAI-MCP-ROOT gate). Populated by
    // the HeaderSessionManager on every initialize (fresh + lazy
    // reconnect); consumed by the tool dispatcher and the hand-written
    // `list_tools` handler.
    let registry = Arc::new(AgentArgumentsRegistry::new());

    let server = ObjectiveAiMcpCli::new(executor, registry.clone());
    let session_manager =
        Arc::new(HeaderSessionManager::new(registry, server.clone()));
    let ct = CancellationToken::new();

    let service = StreamableHttpService::new(move || Ok(server.clone()), session_manager, {
        // rmcp 1.7 marks `StreamableHttpServerConfig` `#[non_exhaustive]`.
        let mut cfg = StreamableHttpServerConfig::default();
        cfg.stateful_mode = true;
        cfg.sse_keep_alive = None;
        cfg.cancellation_token = ct.child_token();
        cfg
    });
    (service, ct)
}
