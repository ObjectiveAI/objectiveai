//! The HTTP stop: `POST /stop`, from loopback only.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::connect_info::ConnectInfo;
use axum::http::StatusCode;

use super::Issuer;

/// `POST /stop`: the issuer told to stop. The rules, as the daemon's
/// and the provider's:
///
/// - Only a loopback peer is heard. The peer's address is the
///   connection's, canonicalised so an IPv4 loopback mapped into IPv6
///   counts, and any other peer is answered `403 Forbidden` with
///   nothing done. Loopback is the whole credential.
/// - A loopback peer is answered `204 No Content`, and the issuer's
///   stop is sent `true` — the same stop the signal sends.
/// - The answer is sent before the drain, not after it, since the
///   drain waits for every request in flight, this one among them.
/// - A second `stop` while the issuer is already draining is `204`
///   too: the stop is a level, not an event.
pub async fn stop(State(issuer): State<Arc<Issuer>>, ConnectInfo(peer): ConnectInfo<SocketAddr>) -> StatusCode {
    if !peer.ip().to_canonical().is_loopback() {
        return StatusCode::FORBIDDEN;
    }
    issuer.stop.send_replace(true);
    StatusCode::NO_CONTENT
}
