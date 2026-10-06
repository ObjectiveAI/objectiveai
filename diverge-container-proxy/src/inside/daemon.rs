//! `/daemon`: the daemon protocol, served to the program and carried
//! to the caller frame by frame.
//!
//! The program is a client of the wire on this socket, as it would be
//! of the daemon itself: it sends client frames, the proxy sends
//! server frames back. Each client frame becomes one daemon channel
//! on the begin scope, carrying the frame; what the caller answers on
//! that channel — the server frames of the frame's scope or channel —
//! is written to the socket as it comes, and a frame the caller could
//! not serve becomes the wire's own word for it: a response finish for
//! a request, a channel response finish for a channel request, nothing
//! for the rest. An auth frame, or a frame that will not decode, ends
//! the connection.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_sdk::shared::containers::daemon;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::server::ServerFrame;
use diverge_sdk::wire::server::channel::Channel;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::answer::{self, Answer};
use crate::ask::{self, Asked};
use crate::encode::encoded;
use crate::own::Own;
use crate::proxy::Proxy;

/// The upgrade. Any number of daemon connections may be open at once.
pub async fn serve(State(proxy): State<Arc<Proxy>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| connection(socket, proxy)).into_response()
}

/// One daemon connection: every client frame off the socket becomes a
/// channel toward the caller, and every answer goes back on the
/// socket through one writer.
async fn connection(socket: WebSocket, proxy: Arc<Proxy>) {
    let (mut sink, mut stream) = socket.split();
    let (out, mut outbox) = mpsc::unbounded_channel::<Vec<u8>>();
    let writer = tokio::spawn(async move {
        while let Some(bytes) = outbox.recv().await {
            if sink.send(Message::Binary(bytes.into())).await.is_err() {
                break;
            }
        }
    });
    let mut tasks = JoinSet::new();
    while let Some(Ok(message)) = stream.next().await {
        let bytes = match message {
            Message::Binary(bytes) => bytes,
            Message::Close(_) => break,
            // A ping, a pong, or text. None of them are this
            // protocol's.
            _ => continue,
        };
        let Ok(request) = daemon::request::Request::decode(&bytes) else {
            break;
        };
        let unserved = unserved(&request);
        let (_begun, channel) = match ask::open(&proxy, Own::Daemon(request)).await {
            Ok(opened) => opened,
            Err(Asked::Encode | Asked::Empty | Asked::Died) => {
                if let Some(frame) = unserved {
                    let _ = out.send(frame);
                }
                continue;
            }
        };
        tasks.spawn(relay(channel, out.clone(), unserved));
    }
    tasks.abort_all();
    writer.abort();
}

/// What the program hears when a frame is not served: the finish of
/// what the frame opened, which the wire already means as "could not
/// serve", and nothing for a frame that opened nothing.
fn unserved(request: &daemon::request::Request<'_>) -> Option<Vec<u8>> {
    match *request {
        daemon::request::Request::Request { scope, .. } => encoded(&ServerFrame::ResponseFinish { scope }),
        daemon::request::Request::ChannelRequest { scope, channel, .. } => {
            encoded(&ServerFrame::ChannelResponseFinish { scope, channel })
        }
        daemon::request::Request::ChannelResponse { .. } | daemon::request::Request::ChannelResponseFinish { .. } => None,
    }
}

/// One channel's answers, written to the socket as server frames
/// until the channel finishes; an error, or a channel that dies with
/// nothing said, is the unserved frame instead.
async fn relay(mut channel: Channel, out: mpsc::UnboundedSender<Vec<u8>>, mut unserved: Option<Vec<u8>>) {
    while let Some(answer) = answer::next(&mut channel).await {
        match answer {
            Answer::Frame(bytes) => match daemon::response::Frame::decode(&bytes) {
                Ok(daemon::response::Frame::Served(served)) => {
                    unserved = None;
                    let Some(frame) = encoded(&ServerFrame::from(served)) else {
                        return;
                    };
                    if out.send(frame).is_err() {
                        return;
                    }
                }
                Ok(daemon::response::Frame::Error(_)) | Err(_) => break,
            },
            Answer::Finish => return,
        }
    }
    if let Some(frame) = unserved {
        let _ = out.send(frame);
    }
}
