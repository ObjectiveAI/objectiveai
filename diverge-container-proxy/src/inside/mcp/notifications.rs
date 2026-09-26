//! The resident notifications ask: one, for the connection's life.

use std::sync::Arc;

use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::shared::mcp;

use crate::answer::{self, Answer};
use crate::ask;
use crate::own::Own;
use crate::proxy::Proxy;

/// Keep one notifications ask open — from the agent's first exchange
/// on — and fan everything that answers it out to the sessions.
///
/// Nothing is asked before the [`Gate`](super::Gate) opens: a
/// container whose agent never makes an MCP exchange never hears a
/// notification, and never costs the caller one. After that the ask
/// is made once, on the one connection there is, and heard until it
/// ends: a finish, an error frame, or a frame this crate could not
/// read is the far side saying "no more notifications", and the
/// channel closing without a finish is the connection gone. There is
/// no next connection to ask on, so nothing is re-asked.
///
/// Each notification is shipped to every registered session, once,
/// as it arrives — see [`Peers::broadcast`](super::Peers::broadcast)
/// for the fan-out and its trade.
pub async fn notifications(proxy: Arc<Proxy>) {
    proxy.gate.opened().await;
    let Ok((_begun, mut channel)) = ask::open(
        &proxy,
        Own::McpNotifications(mcp::notifications::request::Request),
    )
    .await
    else {
        return;
    };
    while let Some(Answer::Frame(bytes)) = answer::next(&mut channel).await {
        match mcp::notifications::response::Frame::decode(&bytes) {
            Ok(mcp::notifications::response::Frame::Notification(notification)) => {
                proxy.peers.broadcast(notification).await;
            }
            Ok(mcp::notifications::response::Frame::Error(_)) | Err(_) => return,
        }
    }
}
