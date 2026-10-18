//! The HTTP stop: `POST /stop`, from loopback only.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::connect_info::ConnectInfo;
use axum::http::StatusCode;

use crate::daemon::Daemon;

/// `POST /stop`: the daemon told to stop, over plain HTTP on its own
/// port, outside the wire. The rules:
///
/// - Only a loopback peer is heard. The peer's address is the
///   connection's, canonicalised so an IPv4 loopback mapped into IPv6
///   counts, and any other peer is answered `403 Forbidden` with
///   nothing done. Loopback is the whole credential: the request
///   carries no body and names no account, and none is read.
/// - A loopback peer is answered `204 No Content`, and the daemon's
///   stop is sent `true` — the same stop the signal sends, so what
///   follows is the Ctrl-C path, in [`run`](super::run)'s order.
/// - The answer is sent before the drain, not after it: a handler
///   that waited for the daemon to end would be a request in flight,
///   and the drain waits for those, so it could never end. The caller
///   reads the close that follows the answer — the listener draining
///   its connections — as the sign the stop was accepted, and the
///   lock file letting go as the sign the teardown ran.
/// - A second `stop` while the daemon is already draining is `204`
///   too: the stop is a level, not an event, and sending `true` again
///   changes nothing.
pub async fn stop(State(daemon): State<Arc<Daemon>>, ConnectInfo(peer): ConnectInfo<SocketAddr>) -> StatusCode {
    if !peer.ip().to_canonical().is_loopback() {
        return StatusCode::FORBIDDEN;
    }
    daemon.stop.send_replace(true);
    StatusCode::NO_CONTENT
}
