//! Client-side handle for sending `client_request::Notify` frames
//! over a streaming WS connection and awaiting their matching
//! `client_response::Response` replies.
//!
//! Returned from every `*_streaming_ws` method alongside the chunk
//! Stream. Drop the Notifier to relinquish the write half — when
//! both the Notifier and the Stream are dropped, the demux task
//! exits and the WS closes cleanly.

use crate::client_objectiveai_mcp::{client_request, client_response};
use futures::SinkExt;
use futures::stream::SplitSink;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, oneshot};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite};

/// Shared sender half of a split WebSocket. Both the Notifier and
/// the demux task (which writes `server_response` frames after
/// handler dispatch) hold a clone; the mutex serializes writes so
/// frames don't interleave on the wire.
pub(crate) type SharedSink = Arc<
    Mutex<
        SplitSink<
            WebSocketStream<MaybeTlsStream<TcpStream>>,
            tungstenite::Message,
        >,
    >,
>;

/// Per-connection registry of outstanding notify ids and the
/// oneshot senders the demux task fulfills when the matching
/// `client_response::Response` arrives.
pub(crate) type PendingNotifies =
    Arc<dashmap::DashMap<String, oneshot::Sender<client_response::Response>>>;

/// Client-side handle for sending notifies to a running agent
/// completion (or any other streaming endpoint) over the same WS
/// that carries the chunk stream.
///
/// Multiple in-flight notifies are supported; each gets a unique id
/// and parks its own oneshot. Calls are independent and can run
/// concurrently from multiple tasks.
#[derive(Clone)]
pub struct Notifier {
    sink: SharedSink,
    pending: PendingNotifies,
}

impl Notifier {
    pub(crate) fn new(sink: SharedSink, pending: PendingNotifies) -> Self {
        Self { sink, pending }
    }

    /// Push a user message at a running agent completion identified
    /// by `params.response_id`. The message surfaces to the model
    /// on its next natural inspection point.
    ///
    /// Returns `Ok(())` if the server accepted the notify. Returns
    /// the server-supplied `code + message` if the server replied
    /// with an `Error` variant (e.g. unknown response_id, MCP
    /// channel down). Returns [`super::HttpError::NotifyChannelClosed`]
    /// if the WS was torn down before the reply arrived.
    pub async fn notify(
        &self,
        params: crate::agent::completions::request::AgentCompletionNotifyParams,
    ) -> Result<(), super::HttpError> {
        self.send(client_request::Payload::AgentCompletionNotify(params))
            .await
    }

    /// Forward a `notifications/{tools,resources}/list_changed`
    /// observation from an upstream `mcp::Connection` up to the API,
    /// which will fan it out as an SSE event on every matching
    /// `/objectiveai-mcp/{ws_session_id}` GET stream subscribed to
    /// the same `mcp_session_id`.
    ///
    /// Same ack semantics as [`Self::notify`].
    pub async fn notify_list_changed(
        &self,
        change: client_request::McpListChanged,
    ) -> Result<(), super::HttpError> {
        self.send(client_request::Payload::McpListChanged(change))
            .await
    }

    /// Common send-and-await-ack body shared by every `notify_*` method.
    async fn send(
        &self,
        payload: client_request::Payload,
    ) -> Result<(), super::HttpError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id.clone(), tx);

        let request = client_request::Request {
            id: id.clone(),
            payload,
        };
        let frame = match serde_json::to_string(&request) {
            Ok(s) => s,
            Err(e) => {
                self.pending.remove(&id);
                return Err(super::HttpError::NotifySerialize(e));
            }
        };

        {
            let mut guard = self.sink.lock().await;
            if let Err(e) =
                guard.send(tungstenite::Message::Text(frame.into())).await
            {
                drop(guard);
                self.pending.remove(&id);
                return Err(super::HttpError::NotifySend(e));
            }
        }

        let response = match rx.await {
            Ok(r) => r,
            Err(_) => return Err(super::HttpError::NotifyChannelClosed),
        };

        match response {
            client_response::Response::Ok { .. } => Ok(()),
            client_response::Response::Error { code, message, .. } => {
                Err(super::HttpError::NotifyRejected { code, message })
            }
        }
    }
}
