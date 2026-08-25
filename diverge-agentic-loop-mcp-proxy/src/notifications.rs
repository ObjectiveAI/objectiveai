//! The resident notifications stream: one channel, always maintained.

use std::sync::Arc;

use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::channel_request;
use diverge_provider_sdk::shared::mcp;

use crate::peers::Peers;
use crate::proxy::{ChannelEvent, Proxy};

/// Maintain the one notifications channel for the proxy's whole life,
/// and fan everything on it out to the sessions.
///
/// The channel is re-opened for as long as the proxy runs; what
/// varies is only when:
///
/// - **The connection died** — the stream ended without a finish —
///   and the next open happens immediately, blocking until the next
///   connection is live, which is where the re-ask belongs anyway.
/// - **The far side ended the stream deliberately** — an error frame,
///   a clean finish, or a frame this crate could not read — which
///   means "no more notifications on THIS connection." Re-asking the
///   same connection would be a refusal to hear that, and a spin; the
///   task waits for the connection to change and opens on the next.
///
/// Each notification is shipped to every registered session, once,
/// as it arrives — see [`Peers::broadcast`] for the fan-out and its
/// trade.
pub async fn run(proxy: Arc<Proxy>, peers: Arc<Peers>) {
    loop {
        let Ok((generation, mut receiver)) = proxy
            .open(channel_request::Frame::McpNotifications(
                mcp::notifications::request::Request,
            ))
            .await
        else {
            // The request would not encode, which cannot change by
            // retrying: it has no params.
            return;
        };

        let deliberate = loop {
            let Some(event) = receiver.recv().await else {
                // Died un-finished: the connection went.
                break false;
            };
            let bytes = match event {
                ChannelEvent::Response(bytes) => bytes,
                ChannelEvent::Finished => break true,
            };
            match mcp::notifications::response::Frame::decode(&bytes) {
                Ok(mcp::notifications::response::Frame::Notification(
                    notification,
                )) => peers.broadcast(notification).await,
                Ok(mcp::notifications::response::Frame::Error(_)) => {
                    break true;
                }
                Err(_) => break true,
            }
        };

        if deliberate {
            proxy.wait_past(generation).await;
        }
    }
}
