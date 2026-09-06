//! The resident notifications ask: one, always maintained.

use std::sync::Arc;

use diverge_provider_sdk::container_proxy::requests::request::Request;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::shared::mcp;

use super::{Gate, Peers};
use crate::requests::{Event, Requests};

/// Keep one notifications ask open for the proxy's whole life — from
/// the agent's first exchange on — and fan everything that answers
/// it out to the sessions.
///
/// Nothing is asked before the [`Gate`] opens: a container whose
/// agent never makes an MCP exchange never hears a notification, and
/// never costs the caller one. After that the ask is re-sent for as
/// long as the proxy runs; what varies is only when:
///
/// - **It died** — `/requests` went before the answer path opened,
///   or the path ended abruptly — and the next ask happens at once,
///   blocking until the next connection is live, which is where the
///   re-ask belongs anyway.
/// - **The far side ended the stream deliberately** — a clean close,
///   an error frame, or a frame this crate could not read — which
///   means "no more notifications on THIS connection." Re-asking the
///   same connection would be a refusal to hear that, and a spin; the
///   task waits for the connection to change and asks on the next.
///
/// Each notification is shipped to every registered session, once,
/// as it arrives — see [`Peers::broadcast`] for the fan-out and its
/// trade.
pub async fn notifications(
    requests: Arc<Requests>,
    peers: Arc<Peers>,
    gate: Arc<Gate>,
) {
    gate.opened().await;
    loop {
        let Ok((generation, mut receiver)) = requests
            .ask(Request::McpNotifications(
                mcp::notifications::request::Request,
            ))
            .await
        else {
            // The ask would not encode, which cannot change by
            // retrying: it has no params.
            return;
        };

        let deliberate = loop {
            match receiver.recv().await {
                Some(Event::Message(bytes)) => {
                    match mcp::notifications::response::Frame::decode(&bytes) {
                        Ok(mcp::notifications::response::Frame::Notification(
                            notification,
                        )) => peers.broadcast(notification).await,
                        Ok(mcp::notifications::response::Frame::Error(_)) => {
                            break true;
                        }
                        Err(_) => break true,
                    }
                }
                Some(Event::Complete) => break true,
                Some(Event::Died) | None => break false,
                // The postgres path's alone; never on this one.
                Some(Event::Opened(_)) => {}
            }
        };

        if deliberate {
            requests.wait_past(generation).await;
        }
    }
}
