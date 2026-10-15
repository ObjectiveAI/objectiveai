//! The port, and the WebSocket accepted on it.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::{IntoResponse, Response};
use diverge_sdk::wire::connection::Connection;
use tokio::net::TcpListener;
use tokio::sync::watch;

use super::{Error, connection};
use crate::daemon::Daemon;

/// Listen on `port`, on every interface, until `stop` says so; every
/// WebSocket upgrade at any path is one [`connection`] served with
/// `daemon`. A port that could not be bound is [`Error::Bind`]; a
/// listener that stopped on its own is [`Error::Serve`].
pub async fn listen(port: u16, mut stop: watch::Receiver<bool>, daemon: Arc<Daemon>) -> Result<(), Error> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await.map_err(Error::Bind)?;
    let router = Router::new().fallback(accept).with_state(daemon);
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async move {
            let _ = stop.wait_for(|stopped| *stopped).await;
        })
        .await
        .map_err(Error::Serve)
}

/// Upgrade, and serve the socket for its life.
async fn accept(
    State(daemon): State<Arc<Daemon>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade
        .on_upgrade(move |socket| connection(Connection::Incoming(socket), peer.ip(), daemon))
        .into_response()
}
