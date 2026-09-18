//! The one connection: the server's scopes, read and dispatched.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::connection::Connection;
use diverge_provider_sdk::container_proxy_endpoints::ClientRequest;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::server::received::Received;
use diverge_provider_sdk::server::session::Session;
use futures_util::StreamExt as _;

use crate::proxy::Proxy;
use crate::{begin, serve};

/// The root: the server's one upgrade. A second, ever, is `409`
/// before the upgrade — there is no second connection to a proxy, and
/// the container is over when the first ends.
pub async fn accept(State(proxy): State<Arc<Proxy>>, upgrade: WebSocketUpgrade) -> Response {
    if !proxy.claim_connection().await {
        return StatusCode::CONFLICT.into_response();
    }
    upgrade
        .on_upgrade(move |socket| connection(socket, proxy))
        .into_response()
}

/// Read the connection for its life: each request the server sends
/// opens a scope, served on a task of its own, never inside this
/// loop — polling the session is what runs the connection, and a
/// scope that waited here would wait on frames nobody reads.
///
/// A credential is a peer speaking some other protocol: type `0` is
/// never sent on this wire, and the connection is closed on it. The
/// session ending is the server gone: every scope on it is over, and
/// the container with them; the tasks are aborted and nothing else is
/// cleaned up.
async fn connection(socket: WebSocket, proxy: Arc<Proxy>) {
    let mut session = Session::new(Connection::Incoming(socket));
    let mut scopes = tokio::task::JoinSet::new();
    while let Some(received) = session.next().await {
        // Finished ones, so the set does not grow for the life of the
        // connection.
        while scopes.try_join_next().is_some() {}
        let (payload, scope) = match received {
            Received::Request(payload, scope) => (payload, scope),
            Received::Auth(_) => break,
        };
        match ClientRequest::decode(&payload).unwrap_or_else(|error| match error {}) {
            ClientRequest::AgentsBegin(frame) => {
                scopes.spawn(begin::agents(Arc::clone(&proxy), scope, frame));
            }
            ClientRequest::ToolsBegin(frame) => {
                scopes.spawn(begin::tools(Arc::clone(&proxy), scope, frame));
            }
            ClientRequest::FuseMount(frame) => {
                scopes.spawn(serve::mount(Arc::clone(&proxy), scope, frame));
            }
            ClientRequest::FilesystemTree(frame) => {
                scopes.spawn(serve::tree(scope, frame));
            }
            ClientRequest::FilesystemRead(frame) => {
                scopes.spawn(serve::read(scope, frame));
            }
            ClientRequest::FilesystemWrite(frame) => {
                scopes.spawn(serve::write(scope, frame));
            }
            // Six scopes have six error vocabularies, and an invalid
            // request names none of them: the finish with nothing
            // before it, which is what the wire means by a request
            // that could not be served.
            ClientRequest::Invalid(_) => {
                scopes.spawn(async move { scope.send_response_finish().await });
            }
        }
    }
    scopes.abort_all();
}
