//! Reverse-channel write primitive used by the JSON-RPC handlers.
//!
//! Every forwarding delegate in [`super::handlers`] re-wraps its call
//! as a `server_request::Request` and ships it through
//! [`send_server_request`]. The recv loop in `streaming_ws.rs` drains
//! `server_response` frames and fulfills the matching oneshots.

use super::registry::{PendingRequests, SharedSink};
use axum::extract::ws::Message;
use futures::SinkExt;
use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
use tokio::sync::oneshot;

/// Register a oneshot under `request.id`, write the request as a text
/// frame, and return the receiver. The caller is responsible for
/// minting the id (and putting it on the request) and applying a
/// timeout (via `tokio::time::timeout`) on the await. On connection
/// drop the recv loop returns and pending oneshots are dropped —
/// receivers observe the close as `Err(RecvError)`.
pub async fn send_server_request(
    sink: &SharedSink,
    pending: &PendingRequests,
    request: server_request::Request,
) -> Result<oneshot::Receiver<server_response::Response>, ()> {
    let id = request.id.clone();
    let (tx, rx) = oneshot::channel();
    pending.insert(id.clone(), tx);

    let frame = match serde_json::to_string(&request) {
        Ok(s) => s,
        Err(_) => {
            pending.remove(&id);
            return Err(());
        }
    };
    let mut guard = sink.lock().await;
    let send_result = guard.send(Message::Text(frame.into())).await;
    if send_result.is_err() {
        drop(guard);
        pending.remove(&id);
        return Err(());
    }
    Ok(rx)
}
