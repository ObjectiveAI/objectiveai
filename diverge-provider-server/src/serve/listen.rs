//! The provider's port: a WebSocket upgrade accepted at any path.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::connection::Connection;
use tokio::net::TcpListener;
use tokio::sync::watch;

use super::{Error, Provider};

/// Listen on `port`, on every interface, until `stop` says so: every
/// request at every path is a WebSocket upgrade, and each upgrade is
/// one connection served by [`Provider::connection`]. A request that
/// is not an upgrade is answered as axum answers one. When `stop`
/// turns `true` the listener closes and what was accepted is left to
/// finish on its own.
pub async fn listen(provider: Arc<Provider>, port: u16, mut stop: watch::Receiver<bool>) -> Result<(), Error> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await.map_err(Error::Bind)?;
    let router = Router::new().fallback(accept).with_state(provider);
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async move {
            let _ = stop.wait_for(|stopped| *stopped).await;
        })
        .await
        .map_err(Error::Serve)
}

/// One upgrade: the peer's address kept for the SDK, and the socket
/// handed over once the upgrade is done. The connection is served in
/// the task axum runs the upgrade in.
async fn accept(
    State(provider): State<Arc<Provider>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade
        .on_upgrade(move |socket| async move {
            let authorization = provider.incoming();
            provider.connection(Connection::Incoming(socket), authorization, peer.ip()).await;
        })
        .into_response()
}
