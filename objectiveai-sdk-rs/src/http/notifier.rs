//! Client-side handle for sending `client_request::Notify` frames
//! over a streaming WS connection and awaiting their matching
//! `client_response::Response` replies.
//!
//! Returned from every `*_streaming_ws` method alongside the chunk
//! Stream. Drop the Notifier to relinquish the write half — when
//! both the Notifier and the Stream are dropped, the demux task
//! exits and the WS closes cleanly.

use crate::client_objectiveai_mcp::{
    client_request, client_response, server_response,
};
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

    /// Forward a `notifications/{tools,resources}/list_changed`
    /// observation from an upstream `mcp::Connection` up to the API,
    /// which will fan it out as an SSE event on every matching
    /// per-MCP GET stream — `/objectiveai` or
    /// `/{owner}/{name}/{ver}/{mcp}`, subscribed under the
    /// per-agent `response_id` + matching `McpKind`.
    ///
    /// Returns `Ok(())` if the server accepted the notify. Returns
    /// the server-supplied `code + message` if the server replied
    /// with an `Error` variant. Returns
    /// [`super::HttpError::NotifyChannelClosed`] if the WS was torn
    /// down before the reply arrived.
    pub async fn notify_list_changed(
        &self,
        change: client_request::McpListChanged,
    ) -> Result<(), super::HttpError> {
        match self
            .send_raw(client_request::Payload::McpListChanged(change))
            .await?
        {
            client_response::Response::Ok { .. } => Ok(()),
            client_response::Response::Error { code, message, .. } => {
                Err(super::HttpError::NotifyRejected { code, message })
            }
            // This call only sends `McpListChanged`, whose reply is
            // `Ok`/`Error`. The MCP-op result variants
            // (`ListTools`/`CallTool`/...) are replies to other requests
            // and shouldn't arrive here (responses correlate by id).
            _ => Err(super::HttpError::NotifyRejected {
                code: 0,
                message: serde_json::Value::String(
                    "unexpected reply variant for notify".to_string(),
                ),
            }),
        }
    }

    /// Run the proxy's aggregated `tools/list` for `response_id` over
    /// this WS and return the proxy's verbatim
    /// [`JsonRpcResult`](crate::client_objectiveai_mcp::server_response::JsonRpcResult).
    /// A server-level rejection (unknown / banned `response_id`) is
    /// folded into `JsonRpcResult::Err` so callers see a single
    /// MCP-shaped outcome; only transport failures surface as `Err`.
    pub async fn list_tools(
        &self,
        response_id: String,
        name: Option<String>,
        params: crate::mcp::tool::ListToolsRequest,
    ) -> Result<
        server_response::JsonRpcResult<crate::mcp::tool::ListToolsResult>,
        super::HttpError,
    > {
        match self
            .send_raw(client_request::Payload::ListTools {
                response_id,
                name,
                params,
            })
            .await?
        {
            client_response::Response::ListTools { result, .. } => Ok(result),
            other => Self::fold_unexpected(other),
        }
    }

    /// Run the proxy's aggregated `tools/call` for `response_id` over
    /// this WS (routes by tool-name prefix to the owning upstream;
    /// does NOT consult the queue delegate). See [`Self::list_tools`]
    /// for the error-folding contract.
    pub async fn call_tool(
        &self,
        response_id: String,
        params: crate::mcp::tool::CallToolRequestParams,
    ) -> Result<
        server_response::JsonRpcResult<crate::mcp::tool::CallToolResult>,
        super::HttpError,
    > {
        match self
            .send_raw(client_request::Payload::CallTool {
                response_id,
                params,
            })
            .await?
        {
            client_response::Response::CallTool { result, .. } => Ok(result),
            other => Self::fold_unexpected(other),
        }
    }

    /// Run the proxy's aggregated `resources/list` for `response_id`
    /// over this WS. See [`Self::list_tools`] for the error contract.
    pub async fn list_resources(
        &self,
        response_id: String,
        name: Option<String>,
        params: crate::mcp::resource::ListResourcesRequest,
    ) -> Result<
        server_response::JsonRpcResult<crate::mcp::resource::ListResourcesResult>,
        super::HttpError,
    > {
        match self
            .send_raw(client_request::Payload::ListResources {
                response_id,
                name,
                params,
            })
            .await?
        {
            client_response::Response::ListResources { result, .. } => {
                Ok(result)
            }
            other => Self::fold_unexpected(other),
        }
    }

    /// List the proxy's connected upstream MCP servers for `response_id`
    /// over this WS (a proxy-local aggregate). See [`Self::list_tools`]
    /// for the error contract.
    pub async fn list_servers(
        &self,
        response_id: String,
    ) -> Result<
        server_response::JsonRpcResult<crate::mcp::server::ListServersResult>,
        super::HttpError,
    > {
        match self
            .send_raw(client_request::Payload::ListServers { response_id })
            .await?
        {
            client_response::Response::ListServers { result, .. } => Ok(result),
            other => Self::fold_unexpected(other),
        }
    }

    /// Run the proxy's `resources/read` for `response_id` over this WS
    /// (routes by URI prefix). See [`Self::list_tools`] for the error
    /// contract.
    pub async fn read_resource(
        &self,
        response_id: String,
        params: crate::mcp::resource::ReadResourceRequestParams,
    ) -> Result<
        server_response::JsonRpcResult<crate::mcp::resource::ReadResourceResult>,
        super::HttpError,
    > {
        match self
            .send_raw(client_request::Payload::ReadResource {
                response_id,
                params,
            })
            .await?
        {
            client_response::Response::ReadResource { result, .. } => {
                Ok(result)
            }
            other => Self::fold_unexpected(other),
        }
    }

    /// Map a non-matching reply for an MCP-op request into the
    /// `JsonRpcResult<R>` the caller expects:
    ///
    /// - generic `Error { code, message }` — a server-level rejection
    ///   (e.g. unknown / banned `response_id`) — folds into
    ///   `JsonRpcResult::Err` so every server-originated outcome is one
    ///   MCP-shaped value.
    /// - any other variant is a correlation/protocol bug (a reply for a
    ///   different request kind reached this oneshot) → `NotifyRejected`.
    fn fold_unexpected<R>(
        other: client_response::Response,
    ) -> Result<server_response::JsonRpcResult<R>, super::HttpError> {
        match other {
            client_response::Response::Error { code, message, .. } => {
                let message = match message {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                Ok(server_response::JsonRpcResult::Err {
                    code: code as i64,
                    message,
                    data: None,
                })
            }
            _ => Err(super::HttpError::NotifyRejected {
                code: 0,
                message: serde_json::Value::String(
                    "unexpected reply variant for mcp request".to_string(),
                ),
            }),
        }
    }

    /// Common send-and-await body: assign a correlation id, park a
    /// oneshot in `pending`, write the framed request, and return the
    /// raw [`client_response::Response`] the demux task routes back by
    /// id. Callers interpret the variant.
    /// Send one UNSOLICITED `server_response` frame on the WS — the
    /// seam the multi-frame `Command` exchange rides: the daemon
    /// streams `Item`/`Error`/`Done` frames for a request id AFTER
    /// `McpHandler::handle` already returned the `Ack` frame. No
    /// pending entry is parked — response frames correlate by id on
    /// the REQUESTER's (proxy's) side.
    pub async fn send_server_response(
        &self,
        response: &server_response::Response,
    ) -> Result<(), super::HttpError> {
        let frame = serde_json::to_string(response)
            .map_err(super::HttpError::NotifySerialize)?;
        let mut guard = self.sink.lock().await;
        guard
            .send(tungstenite::Message::Text(frame.into()))
            .await
            .map_err(super::HttpError::NotifySend)
    }

    async fn send_raw(
        &self,
        payload: client_request::Payload,
    ) -> Result<client_response::Response, super::HttpError> {
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

        match rx.await {
            Ok(r) => Ok(r),
            Err(_) => Err(super::HttpError::NotifyChannelClosed),
        }
    }
}
