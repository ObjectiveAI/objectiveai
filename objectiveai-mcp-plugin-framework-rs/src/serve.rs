//! Serving the plugin: the one call an importer makes and never
//! returns from.
//!
//! ```no_run
//! # use objectiveai_mcp_plugin_framework::{serve, tools::Tools};
//! # use rmcp::handler::server::tool::ToolRouter;
//! # #[derive(Clone)] struct State;
//! # fn tool_router() -> ToolRouter<State> { ToolRouter::new() }
//! # async fn main_() -> Result<std::convert::Infallible, serve::Error> {
//! let tools = Tools::new(tool_router());
//! // Never returns except to fail.
//! serve::serve(serve::Config::new(8080), State, tools).await
//! # }
//! ```
//!
//! **The wire shape is not a choice.** The laboratory host publishes
//! the container's manifest port and dials `http://127.0.0.1:<port>/`
//! with rmcp's streamable-HTTP protocol, session id and all. So the
//! transport is `StreamableHttpService`, the route is `/`, and the
//! only free variable is which port the manifest declared — which is
//! why [`Config::new`] takes it and nothing else does. There is no
//! `PORT` in the environment to read: the host publishes the port the
//! manifest names, so the plugin and its manifest are the two halves
//! that have to agree.

use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

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

use crate::tools::Tools;

/// Everything [`serve`] can fail with. Both are the listener — once
/// serving starts, a failed request is an MCP error, not the end of
/// the server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bind {addr}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("serve")]
    Serve(#[source] std::io::Error),
}

/// How to serve.
#[derive(Debug, Clone)]
pub struct Config {
    /// The port the plugin's manifest declares under `mcp.port`. The
    /// host publishes exactly that port; a mismatch means the host
    /// publishes a port nothing is listening on.
    pub port: u16,
    /// The bind address. `0.0.0.0` by default and almost never worth
    /// changing: podman publishes the CONTAINER's port to a host port,
    /// and a server bound to loopback INSIDE the container is
    /// unreachable through that publish. Binding `127.0.0.1` here is
    /// the classic way to build a plugin that works under `cargo run`
    /// and never once inside a container.
    pub bind: IpAddr,
    /// SSE keep-alive ping interval, or `None` for none.
    pub sse_keep_alive: Option<Duration>,
    /// Whether the server keeps per-session state. TRUE, because the
    /// ObjectiveAI client is a session client — it carries an
    /// `mcp-session-id` and expects the server to remember it.
    pub stateful: bool,
}

impl Config {
    /// The config for a plugin declaring `mcp.port = port`.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            bind: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            sse_keep_alive: Some(Duration::from_secs(15)),
            stateful: true,
        }
    }

    fn addr(&self) -> SocketAddr {
        SocketAddr::new(self.bind, self.port)
    }
}

/// Serve `tools` over `state` until the process ends.
///
/// Returns only on failure — a bind that did not take, or a listener
/// that died. Every lesser problem is an MCP error on one request.
///
/// `state` is what each tool receives as `&S`: rmcp's tool router is
/// generic over the service type, so the tools and the state they run
/// against are one unit and cannot be passed separately.
pub async fn serve<S>(config: Config, state: S, tools: Tools<S>) -> Result<Infallible, Error>
where
    S: Send + Sync + 'static,
{
    serve_handler(config, Handler::new(state, tools)).await
}

/// [`serve`] for a plugin that implements [`ServerHandler`] itself —
/// the way in for prompts, resources, or a custom `get_info`, none of
/// which the plain [`Handler`] exposes.
///
/// The handler is CLONED per session, which is what rmcp's transport
/// asks for. Keep anything shared behind an `Arc` (as [`Tools`]
/// already is) so clones stay cheap and see each other's changes.
pub async fn serve_handler<H>(config: Config, handler: H) -> Result<Infallible, Error>
where
    H: ServerHandler + Clone + Send + Sync + 'static,
{
    // `#[non_exhaustive]`, so assign rather than construct — which
    // also means a future rmcp field arrives at ITS default here
    // instead of failing the build.
    let mut http = StreamableHttpServerConfig::default();
    http.sse_keep_alive = config.sse_keep_alive;
    http.stateful_mode = config.stateful;
    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        Arc::new(LocalSessionManager::default()),
        http,
    );
    // At the ROOT, because that is what the host dials — see the
    // module docs.
    let router = axum::Router::new().fallback_service(service);

    let addr = config.addr();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|source| Error::Bind { addr, source })?;
    axum::serve(listener, router)
        .await
        .map_err(Error::Serve)?;
    // `axum::serve` only returns on error without a shutdown signal,
    // and none is installed — the container's life is the server's.
    Err(Error::Serve(std::io::Error::other(
        "the listener stopped without an error",
    )))
}

/// The [`ServerHandler`] [`serve`] builds: tools and nothing else.
///
/// A plugin that needs more than tools — prompts, resources, a
/// `get_info` of its own — implements [`ServerHandler`] directly and
/// calls [`serve_handler`]. This exists so that the common case does
/// not have to.
pub struct Handler<S> {
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

impl<S> Handler<S> {
    pub fn new(state: S, tools: Tools<S>) -> Self {
        Self {
            state: Arc::new(state),
            tools,
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
        // The tool router hands `&S` to each tool, so the context
        // borrows the STATE — not this handler.
        let context = ToolCallContext::new(self.state.as_ref(), request, context);
        self.tools.call(context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bind_is_not_loopback() {
        // A loopback bind inside a container is unreachable through
        // podman's port publish, so the default must not be one.
        assert!(!Config::new(8080).bind.is_loopback());
        assert_eq!(Config::new(8080).port, 8080);
    }

    #[test]
    fn stateful_by_default() {
        // The ObjectiveAI client carries an `mcp-session-id`.
        assert!(Config::new(1).stateful);
    }
}
