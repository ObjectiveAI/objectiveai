//! The HTTP stop: `POST /stop`, from loopback only.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::connect_info::ConnectInfo;
use axum::http::StatusCode;

use super::Provider;

/// `POST /stop`: the provider told to stop, over plain HTTP on its
/// own port, outside the protocol. The rules:
///
/// - Only a loopback peer is heard. The peer's address is the
///   connection's, canonicalised so an IPv4 loopback mapped into IPv6
///   counts, and any other peer is answered `403 Forbidden` with
///   nothing done. Loopback is the whole credential: the request
///   carries no body and names no identity, and none is read.
/// - A loopback peer is answered `204 No Content`, and the provider's
///   stop is sent `true` — the same stop the signal sends, so what
///   follows is the signal's path, in [`run`](super::run)'s order.
/// - The answer is sent before the drain, not after it: a handler
///   that waited for the provider to end would be a request in
///   flight, and the drain waits for those, so it could never end.
///   The caller reads the close that follows the answer — the
///   listener draining its connections — as the sign the stop was
///   accepted, and the lock file letting go as the sign the teardown
///   ran.
/// - A second `stop` while the provider is already draining is `204`
///   too: the stop is a level, not an event, and sending `true` again
///   changes nothing.
pub async fn stop(State(provider): State<Arc<Provider>>, ConnectInfo(peer): ConnectInfo<SocketAddr>) -> StatusCode {
    if !peer.ip().to_canonical().is_loopback() {
        return StatusCode::FORBIDDEN;
    }
    provider.stop.send_replace(true);
    StatusCode::NO_CONTENT
}
