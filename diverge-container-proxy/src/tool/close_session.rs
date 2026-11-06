//! A caller's MCP session, closed.

use diverge_sdk::shared::mcp::close_session::{request, response};
use diverge_sdk::shared::mcp::session;
use diverge_sdk::shared::mcp::session::Sessioned;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::encode::encoded;
use crate::proxy::Proxy;
use crate::reply::reply;

/// The session under the caller's key removed and closed
/// ([`Sessions::close`](super::Sessions::close)), then `Closed` and the
/// finish. The rules:
///
/// 1. A key with no session open is `Closed` all the same: the caller
///    asked for a state that already holds.
/// 2. It is not the container's activity: it enters no gate and moves
///    the container neither active nor inactive, since it reaches no
///    server.
pub async fn close_session(proxy: &Proxy, scope: &ScopeHandle, channel: u32, request: Sessioned<request::Request>) {
    proxy.sessions.close(&session::key(&request.session)).await;
    reply(scope, channel, encoded(&response::Frame::Closed)).await;
}
