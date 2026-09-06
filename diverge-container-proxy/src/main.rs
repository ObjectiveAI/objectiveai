//! The proxy inside a Diverge container.
//!
//! One program beside every container's own entrypoint, serving the
//! SDK's [`container_proxy`]
//! wire on port `14979`: the container's asks go out on `/requests`,
//! the server answers each on the ask's own path, and the paths the
//! server opens on its own — the filetree, a file read, a file write
//! — are served here too.
//!
//! Built one feature at a time. Today: MCP. To the agent beside it
//! the proxy is a fully compliant MCP server at `/mcp/agent`; every
//! exchange the agent asks of it becomes an ask on `/requests`,
//! answered on `/mcp/list-tools/{channel}` and its siblings by the
//! caller's own servers on the far side of the provider.

mod mcp;
mod requests;
mod ws;

use std::sync::Arc;

use diverge_provider_sdk::container_proxy;
use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(run());
}

async fn run() {
    let requests = Arc::new(requests::Requests::new());
    let peers = Arc::new(mcp::Peers::new());

    tokio::spawn(mcp::notifications(
        Arc::clone(&requests),
        Arc::clone(&peers),
    ));

    let agent = StreamableHttpService::new(
        {
            let requests = Arc::clone(&requests);
            let peers = Arc::clone(&peers);
            move || {
                Ok(mcp::Handler {
                    requests: Arc::clone(&requests),
                    peers: Arc::clone(&peers),
                })
            }
        },
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    let app = axum::Router::new()
        .route("/requests", axum::routing::any(ws::requests))
        .route(
            "/mcp/list-tools/{channel}",
            axum::routing::any(ws::mcp_list_tools),
        )
        .route(
            "/mcp/list-resources/{channel}",
            axum::routing::any(ws::mcp_list_resources),
        )
        .route(
            "/mcp/call-tool/{channel}",
            axum::routing::any(ws::mcp_call_tool),
        )
        .route(
            "/mcp/read-resource/{channel}",
            axum::routing::any(ws::mcp_read_resource),
        )
        .route(
            "/mcp/notifications/{channel}",
            axum::routing::any(ws::mcp_notifications),
        )
        .nest_service("/mcp/agent", agent)
        .with_state(Arc::clone(&requests));

    let listener =
        tokio::net::TcpListener::bind(("0.0.0.0", container_proxy::PORT))
            .await
            .expect("the port could not be bound");
    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}
