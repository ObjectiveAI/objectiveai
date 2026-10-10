//! Connecting to another daemon, and carrying the connection.

use std::fmt;

use bytes::Bytes;
use futures_util::StreamExt as _;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::watch;

use super::super::super::server::response;
use super::super::request;
use super::Connected;
use crate::shared::error::Error;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::client::router::Router;
use crate::wire::connection::Connection;
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Connect to another daemon through the provider, and hold the
/// connection.
///
/// The request goes out, the main stream's first frame decides — the
/// connection open, or the provider's error, or a finish with nothing
/// before it — and then the connection is a
/// [`Connection::Local`](crate::wire::connection::Connection::Local)
/// of this end's, split into the frame-level client's router and
/// handle as a socket would be: every response after the first is a
/// server frame of the acceptor's, fed to the router; every client
/// frame the handle writes is a channel response on the one channel
/// the provider opens, the first of which is waited for. What comes
/// back is the [`Connected`]: the handle, and the end.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<Connected, ExecuteError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
    let mut scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let bytes = scope.response_receiver.recv().await.ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(ExecuteError::Frame)?;
    let payload = match envelope {
        frame::server::ServerFrame::Response { payload, .. } => payload,
        frame::server::ServerFrame::ResponseFinish { .. } => return Err(ExecuteError::Unanswered),
        _ => return Err(ExecuteError::Misrouted),
    };
    match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Connected => {}
        response::Frame::Error(error) => return Err(ExecuteError::Provider(error)),
        response::Frame::Frame(_) => return Err(ExecuteError::Misrouted),
    }
    let (to_router, incoming) = mpsc::unbounded_channel::<Bytes>();
    let (outgoing, from_handle) = mpsc::unbounded_channel::<Bytes>();
    let (sink, stream) = Connection::Local { incoming, outgoing }.split();
    let (registration_sender, registration_receiver) = mpsc::unbounded_channel();
    let (closed_sender, closed_receiver) = mpsc::unbounded_channel();
    let router = Router::new(stream, registration_receiver, closed_sender);
    let daemon = Handle::new(sink, registration_sender, closed_receiver);
    let (ended, ended_receiver) = watch::channel(false);
    tokio::spawn(inward(scope.response_receiver, to_router));
    tokio::spawn(outward(scope.request_receiver, from_handle, handle.clone(), scope.scope));
    tokio::spawn(async move {
        let _ = router.run().await;
        ended.send_replace(true);
    });
    Ok(Connected::new(daemon, ended_receiver))
}

/// Every response after the first, the acceptor's server frames, fed
/// to the router whole; the finish, or the connection going, ends
/// the feed, which ends the router.
async fn inward(mut responses: UnboundedReceiver<Bytes>, to_router: mpsc::UnboundedSender<Bytes>) {
    while let Some(bytes) = responses.recv().await {
        match frame::server::ServerFrame::decode(&bytes) {
            Ok(frame::server::ServerFrame::Response { payload, .. }) => match response::Frame::decode(payload) {
                Ok(response::Frame::Frame(inner)) => {
                    if to_router.send(bytes.slice_ref(inner)).is_err() {
                        break;
                    }
                }
                Ok(_) | Err(_) => {}
            },
            _ => break,
        }
    }
}

/// The one channel the provider opens, waited for; then every client
/// frame the handle writes, sent on it as a channel response, and
/// the channel finished when the handle is gone.
async fn outward(mut requests: UnboundedReceiver<Bytes>, mut from_handle: UnboundedReceiver<Bytes>, provider: Handle, scope: u32) {
    let channel = loop {
        let Some(bytes) = requests.recv().await else {
            return;
        };
        if let Ok(frame::server::ServerFrame::ChannelRequest { channel, .. }) = frame::server::ServerFrame::decode(&bytes) {
            break channel;
        }
    };
    while let Some(bytes) = from_handle.recv().await {
        if provider.send_channel_response(scope, channel, &bytes).await.is_err() {
            return;
        }
    }
    let _ = provider.send_channel_response_finish(scope, channel).await;
}

/// A connection that never opened.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The connection ended before anything came back.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it: the provider could
    /// not serve the connect at all.
    Unanswered,
    /// A frame arrived that does not belong first on the main stream.
    Misrouted,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// The provider refused: no daemon accepts under the identity, or
    /// it declined.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => write!(f, "daemons connect request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the connect answered"),
            ExecuteError::Frame(error) => write!(f, "daemons connect answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the connect finished without an answer"),
            ExecuteError::Misrouted => f.write_str("a frame arrived that does not belong on the main stream"),
            ExecuteError::Response(error) => write!(f, "daemons connect answer did not parse: {error}"),
            ExecuteError::Provider(_) => f.write_str("the provider refused the connect"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed | ExecuteError::Unanswered | ExecuteError::Misrouted | ExecuteError::Provider(_) => None,
        }
    }
}
