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
//! // Keep a clone to change the tool set later; `replace` on it is
//! // picked up here without the plugin touching the server.
//! let handle = tools.clone();
//!
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

use arc_swap::{ArcSwap, ArcSwapOption};
use rmcp::handler::server::tool::{ToolCallContext, ToolRoute, ToolRouter};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, ServerInfo,
};
use rmcp::service::{NotificationContext, Peer, RequestContext};
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
/// `state` is what every tool receives as its `&S`: rmcp's router is
/// generic over the service type, so the routes and the value they run
/// against are two halves of one thing.
///
/// Calling [`Tools::replace`] on a clone of `tools`, at any time, both
/// changes what this server routes and tells the client to re-list.
/// The plugin never implements `ServerHandler` to get that.
pub async fn serve<S>(
    config: Config,
    state: S,
    tools: Arc<Tools<S>>,
) -> Result<Infallible, std::io::Error>
where
    S: Send + Sync + 'static,
{
    // The live router, rebuilt on every replace and read wait-free per
    // request.
    let router = Arc::new(ArcSwap::from_pointee(build(&tools.routes())));
    // Filled on `initialize`. One slot: a plugin container serves one
    // completion for one connector, so the newest session is the
    // session. Were several ever to attach, all would route correctly
    // and only the newest would be TOLD to re-list.
    let peer: Arc<ArcSwapOption<Peer<RoleServer>>> = Arc::new(ArcSwapOption::empty());

    // The wiring the module exists for. Note it captures `router` and
    // `peer` — never `tools` — or the closure and the structure
    // holding it would own each other (see `tools::Notifier`).
    let installed = tools.on_replace({
        let (router, peer) = (router.clone(), peer.clone());
        move |routes: Arc<Vec<ToolRoute<S>>>| {
            // Rebuild and publish FIRST: a client that re-lists the
            // instant it is notified must not race the swap.
            router.store(Arc::new(build(&routes)));
            // Then tell it, off this thread — `replace` is sync and
            // has no business awaiting a network write.
            if let Some(peer) = peer.load_full() {
                tokio::spawn(async move {
                    let _ = peer.notify_tool_list_changed().await;
                });
            }
        }
    });
    if !installed {
        return Err(std::io::Error::other(
            "these Tools are already being served — one server per Tools, \
             or the other one would go stale",
        ));
    }

    let handler = Handler {
        state: Arc::new(state),
        router,
        peer,
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
        // Cloned per session; everything inside is shared, so every
        // session sees the same tools and the same swaps.
        move || Ok(handler.clone()),
        Arc::new(LocalSessionManager::default()),
        http,
    );

    // At the ROOT, because that is what the host dials.
    let axum_router = axum::Router::new().fallback_service(service);
    // `0.0.0.0`: see the module docs — loopback here is unreachable
    // through podman's port publish.
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| std::io::Error::new(e.kind(), format!("bind {addr}: {e}")))?;
    axum::serve(listener, axum_router).await?;
    // Unreachable in practice: no shutdown signal is installed, so the
    // container's life is the server's.
    Err(std::io::Error::other("the listener stopped without an error"))
}

/// A [`ToolRouter`] over a route list.
///
/// `add_route` keys by each route's own name, so a duplicate name in
/// the list keeps the LAST one — the same last-wins rmcp's own `merge`
/// has.
fn build<S: Send + Sync + 'static>(routes: &[ToolRoute<S>]) -> ToolRouter<S> {
    let mut router = ToolRouter::new();
    for route in routes {
        router.add_route(route.clone());
    }
    router
}

/// The [`ServerHandler`] [`serve`] runs: tools, and nothing else.
///
/// Private because a plugin has no reason to hold one — `serve` builds
/// it and owns it for the process's life.
struct Handler<S> {
    state: Arc<S>,
    router: Arc<ArcSwap<ToolRouter<S>>>,
    peer: Arc<ArcSwapOption<Peer<RoleServer>>>,
}

impl<S> Clone for Handler<S> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            router: self.router.clone(),
            peer: self.peer.clone(),
        }
    }
}

impl<S: Send + Sync + 'static> ServerHandler for Handler<S> {
    fn get_info(&self) -> ServerInfo {
        // `list_changed` advertised because the tool set really can
        // change: a client that believed otherwise would go on calling
        // a tool that is gone.
        let mut tools = rmcp::model::ToolsCapability::default();
        tools.list_changed = Some(true);
        let mut info = ServerInfo::default();
        info.capabilities = rmcp::model::ServerCapabilities::builder()
            .enable_tools_with(tools)
            .build();
        info
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        // The only way to reach the client later. Until this lands, a
        // `replace` still swaps the routes correctly — it just has
        // nobody to notify yet.
        self.peer.store(Some(Arc::new(context.peer)));
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: self.router.load().list_all(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        // `load_full` rather than `load`: the guard would otherwise be
        // held across the call, and a tool can run for a long time.
        let router = self.router.load_full();
        // The router hands `&S` to each tool, so the context borrows
        // the STATE — not this handler.
        let context = ToolCallContext::new(self.state.as_ref(), request, context);
        router.call(context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Tool;

    struct Server;

    fn route(name: &'static str) -> ToolRoute<Server> {
        let schema: Arc<rmcp::model::JsonObject> = Arc::new(Default::default());
        ToolRoute::new_dyn(Tool::new(name, "", schema), |_context| {
            Box::pin(async { Ok(CallToolResult::success(vec![])) })
        })
    }

    /// A plugin bound to loopback is unreachable through podman's port
    /// publish, so the address `serve` builds must not be one.
    #[test]
    fn bind_address_is_not_loopback() {
        let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080));
        assert!(!addr.ip().is_loopback());
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn build_routes_by_name() {
        let router = build(&[route("alpha"), route("beta")]);
        let names: Vec<_> = router.list_all().into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["alpha", "beta"]);
        assert!(router.get("alpha").is_some());
        assert!(router.get("missing").is_none());
    }

    /// Last-wins on a duplicated name, matching `add_route` and rmcp's
    /// own `merge` — not an error, and not the first one kept.
    #[test]
    fn a_duplicate_name_keeps_the_last_route() {
        let router = build(&[route("alpha"), route("alpha")]);
        assert_eq!(router.list_all().len(), 1);
    }

    /// The swap `serve` installs, exercised without a live session:
    /// replace rebuilds what a request would route against.
    #[test]
    fn replace_rebuilds_the_live_router() {
        let tools = crate::tools::Tools::new([route("alpha")]);
        let router = Arc::new(ArcSwap::from_pointee(build(&tools.routes())));
        let installed = tools.on_replace({
            let router = router.clone();
            move |routes| router.store(Arc::new(build(&routes)))
        });
        assert!(installed);

        assert!(router.load().get("alpha").is_some());
        tools.replace([route("beta")]);
        assert!(router.load().get("beta").is_some(), "the new tool routes");
        assert!(router.load().get("alpha").is_none(), "the old one does not");
    }
}
