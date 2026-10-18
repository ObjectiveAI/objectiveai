//! Opening an accept, and answering every connection it announces.

use std::fmt;
use std::sync::Arc;

use futures_util::StreamExt as _;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use super::super::super::server::{channel_request as announced, response};
use super::super::{channel_request, request};
use super::Accepting;
use crate::provider::client::Acceptor;
use crate::provider::endpoints::containers::client::encoded;
use crate::shared::containers::daemon;
use crate::shared::error::Error;
use crate::wire::client::handle::{Handle, SendError};
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame;

/// Accept daemon connections through the provider, answering each
/// through `acceptor`.
///
/// The request goes out, the main stream's first frame decides — the
/// identity the provider knows this daemon by, or the provider's
/// error, or a finish with nothing before it — and then a task reads
/// every channel the provider opens on the scope, each a connection
/// announced, and answers it: the acceptor is asked, and on a yes
/// this end opens its own half and pumps both directions — the
/// connector's client frames into the acceptor's session, the
/// session's server frames back onto the provider's half — until
/// either side hangs up. What comes back is the [`Accepting`]: the
/// identity, the stop, and the end.
pub async fn execute<A: Acceptor + 'static>(handle: &Handle, acceptor: Arc<A>) -> Result<Accepting, ExecuteError> {
    let mut payload = Vec::new();
    request::Frame
        .encode(&mut Writer::new(&mut payload))
        .unwrap_or_else(|error| match error {});
    let mut scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let bytes = scope.response_receiver.recv().await.ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(ExecuteError::Frame)?;
    let payload = match envelope {
        frame::server::ServerFrame::Response { payload, .. } => payload,
        frame::server::ServerFrame::ResponseFinish { .. } => return Err(ExecuteError::Unanswered),
        _ => return Err(ExecuteError::Misrouted),
    };
    let identity = match response::Frame::decode(payload).map_err(ExecuteError::Response)? {
        response::Frame::Accepting(accepting) => accepting.identity,
        response::Frame::Error(error) => return Err(ExecuteError::Provider(error)),
    };
    tokio::spawn(announcements(scope.request_receiver, handle.clone(), scope.scope, acceptor));
    Ok(Accepting::new(identity, handle.clone(), scope.scope, scope.response_receiver))
}

/// Every channel the provider opens on the scope, answered on a task
/// of its own, until the scope's request stream ends.
async fn announcements<A: Acceptor + 'static>(mut requests: UnboundedReceiver<bytes::Bytes>, handle: Handle, scope: u32, acceptor: Arc<A>) {
    while let Some(bytes) = requests.recv().await {
        let Ok(frame::server::ServerFrame::ChannelRequest { channel, payload, .. }) = frame::server::ServerFrame::decode(&bytes) else {
            continue;
        };
        let Ok(announced::Frame(connection)) = announced::Frame::decode(payload) else {
            let handle = handle.clone();
            tokio::spawn(async move {
                let _ = handle.send_channel_response_finish(scope, channel).await;
            });
            continue;
        };
        tokio::spawn(connection_answer(handle.clone(), scope, channel, connection, Arc::clone(&acceptor)));
    }
}

/// One connection, answered: the acceptor asked; on a yes this end's
/// half opened, quoting the id, and both directions pumped until
/// either ends; on a no, or a half that cannot open, the provider's
/// half finished with nothing.
async fn connection_answer<A: Acceptor>(handle: Handle, scope: u32, channel: u32, connection: crate::shared::daemons::Connection, acceptor: Arc<A>) {
    let connection_id = connection.connection_id;
    let (from_connector, receiver) = mpsc::unbounded_channel();
    let Some(from_daemon) = acceptor.accept(connection, receiver).await else {
        let _ = handle.send_channel_response_finish(scope, channel).await;
        return;
    };
    let mut from_daemon = std::pin::pin!(from_daemon);
    let Some(half) = encoded(&channel_request::Frame::Connection(daemon::request::Daemon { connection_id })) else {
        let _ = handle.send_channel_response_finish(scope, channel).await;
        return;
    };
    let Ok(mut own) = handle.send_channel_request(scope, &half).await else {
        let _ = handle.send_channel_response_finish(scope, channel).await;
        return;
    };
    let forward = tokio::spawn(async move {
        while let Some(bytes) = own.response_receiver.recv().await {
            match frame::server::ServerFrame::decode(&bytes) {
                Ok(frame::server::ServerFrame::ChannelResponse { payload, .. }) => {
                    let Ok(frame) = daemon::client::Frame::decode(payload) else {
                        break;
                    };
                    if from_connector.send(daemon::client::Owned::from(frame)).is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
    });
    while let Some(frame) = from_daemon.next().await {
        let Some(bytes) = encoded(&frame.as_frame()) else {
            break;
        };
        if handle.send_channel_response(scope, channel, &bytes).await.is_err() {
            break;
        }
    }
    let _ = handle.send_channel_response_finish(scope, channel).await;
    let _ = forward.await;
}

/// An accept that never opened.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    Send(SendError),
    /// The connection ended before anything came back.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it: the provider could
    /// not serve the accept at all.
    Unanswered,
    /// A frame arrived that does not belong on the main stream.
    Misrouted,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// The provider refused: the identity accepts here already.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the accept answered"),
            ExecuteError::Frame(error) => write!(f, "daemons accept answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the accept finished without an answer"),
            ExecuteError::Misrouted => f.write_str("a frame arrived that does not belong on the main stream"),
            ExecuteError::Response(error) => write!(f, "daemons accept answer did not parse: {error}"),
            ExecuteError::Provider(_) => f.write_str("the provider refused the accept"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed | ExecuteError::Unanswered | ExecuteError::Misrouted | ExecuteError::Provider(_) => None,
        }
    }
}
