//! The proxy inside a Diverge container.
//!
//! One program beside every container's own entrypoint, speaking the
//! SDK's [`container_proxy_endpoints`] wire with the provider's
//! server over ONE WebSocket, dialled once on `14979` for the
//! container's life. On it the server opens scopes — the begin, each
//! FUSE mount, every tree, read and write — and the proxy answers
//! each; on the begin scope the proxy opens channels of its own for
//! everything the container asks of the world outside: its database
//! connections, its commands, its vault, its tool calls outward.
//!
//! The program beside the proxy sees none of that. On the loopback
//! at `80` it finds a fully compliant MCP server at `/mcp`, the vault
//! as plain HTTP at `/vault/<op>`, and its commands at `POST
//! /command`; at `81` a pgwire listener that is, to its driver, a
//! database. Each of those becomes one channel on the begin scope,
//! and nothing the program dials has to know.
//!
//! For either kind of container the proxy registers the arguments
//! with the program's own server on the loopback when the server
//! begins, hands the server the tools the program answered with, and
//! asks the program for their schema when the server asks. For an
//! agent container the proxy is also the loop's keeper: it holds the
//! queue every enqueue joins, starts a run when none runs and offers
//! each message to the run in flight when one does, and relays every
//! chunk the agent says onto the begin scope's main stream. For a
//! tool container it is the caller's MCP client, one exchange per
//! channel the server opens.
//!
//! [`container_proxy_endpoints`]: diverge_provider_sdk::container_proxy_endpoints

mod agent;
mod answer;
mod ask;
mod begin;
mod connection;
mod encode;
mod filesystem;
mod inside;
mod own;
mod paths;
mod program;
mod proxy;
mod reply;
mod serve;
mod tool;

use std::future::IntoFuture as _;
use std::sync::Arc;

use diverge_container_proxy_sdk::{INSIDE_PORT, POSTGRES_LOOPBACK_PORT};
use diverge_provider_sdk::container_proxy_endpoints::OUTSIDE_PORT;
use futures_util::future;
use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use tokio::net::TcpListener;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(run());
}

async fn run() {
    let proxy = Arc::new(proxy::Proxy::new());

    let mcp = StreamableHttpService::new(
        {
            let proxy = Arc::clone(&proxy);
            move || Ok(inside::mcp::Handler::new(Arc::clone(&proxy)))
        },
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    // The server's side: the one connection, at the root.
    let outside = axum::Router::new()
        .route("/", axum::routing::any(connection::accept))
        .with_state(Arc::clone(&proxy));

    // The program's side: what the program beside the proxy dials.
    let inside = axum::Router::new()
        .route("/vault/get", axum::routing::post(inside::vault::get))
        .route("/vault/set", axum::routing::post(inside::vault::set))
        .route("/vault/delete", axum::routing::post(inside::vault::delete))
        .route("/vault/lock", axum::routing::post(inside::vault::lock))
        .route("/vault/unlock", axum::routing::post(inside::vault::unlock))
        .route("/command", axum::routing::post(inside::command::agent))
        .nest_service("/mcp", mcp)
        .with_state(Arc::clone(&proxy));

    // All three listeners or none: a proxy that could answer asks but
    // not take the driver's connections, or serve the server but not
    // the program, would be a proxy that is sometimes there.
    let (outside_listener, inside_listener, loopback) = future::try_join3(
        TcpListener::bind(("0.0.0.0", OUTSIDE_PORT)),
        TcpListener::bind(("127.0.0.1", INSIDE_PORT)),
        TcpListener::bind(("127.0.0.1", POSTGRES_LOOPBACK_PORT)),
    )
    .await
    .expect("a port could not be bound");

    tokio::spawn(inside::postgres::accept(loopback, Arc::clone(&proxy)));

    future::try_join(
        axum::serve(outside_listener, outside).into_future(),
        axum::serve(inside_listener, inside).into_future(),
    )
    .await
    .expect("the server stopped unexpectedly");
}
