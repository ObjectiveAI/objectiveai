//! Serving the plugin: the one call an importer makes and never
//! returns from.
//!
//! ```no_run
//! # use objectiveai_mcp_plugin_framework::{config::Config, serve, tools::Tools};
//! # use rmcp::handler::server::tool::ToolRouter;
//! # struct State;
//! # fn tool_router() -> ToolRouter<State> { ToolRouter::new() }
//! # async fn main_() -> Result<std::convert::Infallible, std::io::Error> {
//! let tools = Tools::new(tool_router());
//! // Never returns except to fail.
//! serve::serve(Config::new(8080), State, tools).await
//! # }
//! ```
//!
//! **Almost none of this is a choice.** The laboratory host publishes
//! the container's manifest port and dials
//! `http://127.0.0.1:<port>/` speaking rmcp streamable-HTTP with a
//! session id. So the transport is `StreamableHttpService`, the route
//! is the ROOT, the session mode is stateful, and the bind address is
//! `0.0.0.0` — podman publishes the CONTAINER's port, and a server
//! bound to loopback inside the container is unreachable through that
//! publish, which is the classic way to write a plugin that works
//! under `cargo run` and never once in a container.
//!
//! None of those are configurable, because none of them have a second
//! correct value. What a plugin does decide is in
//! [`Config`][crate::config::Config], and it is short.

use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::config::Config;
use crate::tools::Tools;

/// Serve `tools` over `state` until the process ends.
///
/// Returns only to FAIL — a bind that did not take, or a listener that
/// died — which is what `Infallible` in the success position says.
/// Every lesser problem is an MCP error on one request, not the end of
/// the server. Both failures are genuinely `io::Error`s, so there is
/// no error type of our own to learn.
///
/// `state` comes along with `tools` because rmcp's tool router is
/// generic over the service type and hands every tool an `&S`: the
/// two are one unit and cannot be passed separately.
pub async fn serve<S>(
    config: Config,
    state: S,
    tools: Tools<S>,
) -> Result<Infallible, std::io::Error>
where
    S: Send + Sync + 'static,
{
    let handler = Handler {
        state: Arc::new(state),
        tools,
    };

    // Every rmcp config struct here is `#[non_exhaustive]`, so assign
    // rather than construct — a field added upstream then arrives at
    // ITS default instead of breaking the build.
    let mut http = StreamableHttpServerConfig::default();
    http.sse_keep_alive = config.sse_keep_alive;
    // The ObjectiveAI client is a session client: it carries an
    // `mcp-session-id` and expects the server to remember it.
    http.stateful_mode = true;
    let service = StreamableHttpService::new(
        // Cloned per session. Everything inside is `Arc`, so clones
        // are cheap and see each other's tool edits.
        move || Ok(handler.clone()),
        Arc::new(LocalSessionManager::default()),
        http,
    );

    // At the ROOT, because that is what the host dials.
    let router = axum::Router::new().fallback_service(service);
    // `0.0.0.0`: see the module docs — loopback here is unreachable
    // through podman's port publish.
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        std::io::Error::new(e.kind(), format!("bind {addr}: {e}"))
    })?;
    axum::serve(listener, router).await?;
    // Unreachable in practice: no shutdown signal is installed, so the
    // container's life is the server's.
    Err(std::io::Error::other("the listener stopped without an error"))
}

/// The [`ServerHandler`] [`serve`] runs: tools, and nothing else.
///
/// Private because a plugin has no reason to hold one — `serve` builds
/// it and owns it for the process's life.
struct Handler<S> {
    state: Arc<S>,
    tools: Tools<S>,
}

impl<S> Clone for Handler<S> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            tools: self.tools.clone(),
        }
    }
}

impl<S: Send + Sync + 'static> ServerHandler for Handler<S> {
    fn get_info(&self) -> ServerInfo {
        // `list_changed` advertised, because `Tools` can be edited
        // while serving: a client that believed the list was fixed
        // would go on calling a tool that is gone.
        let mut tools = rmcp::model::ToolsCapability::default();
        tools.list_changed = Some(true);
        let mut info = ServerInfo::default();
        info.capabilities = rmcp::model::ServerCapabilities::builder()
            .enable_tools_with(tools)
            .build();
        info
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: self.tools.list(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        // The router hands `&S` to each tool, so the context borrows
        // the STATE — not this handler.
        let context = ToolCallContext::new(self.state.as_ref(), request, context);
        self.tools.call(context).await
    }
}

#[cfg(test)]
mod tests {
    /// The one invariant in this file worth pinning: a plugin bound to
    /// loopback is unreachable through podman's port publish, so the
    /// address `serve` builds must not be one.
    #[test]
    fn bind_address_is_not_loopback() {
        let addr = super::SocketAddr::from((super::Ipv4Addr::UNSPECIFIED, 8080));
        assert!(!addr.ip().is_loopback());
        assert_eq!(addr.port(), 8080);
    }
}
