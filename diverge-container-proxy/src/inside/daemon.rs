//! The daemon, from the inside: the WebSocket at `/daemon`, each
//! connection announced and then carried.
//!
//! The proxy is the daemon to the container. A program that dials
//! `ws://127.0.0.1:80/daemon` gets a connection that is one ask on the
//! begin scope — `Daemon`, with an id the proxy minted — and, once the
//! server opens its half, a conduit: the program's client frames out
//! as responses on the server's channel, the ask's frames in as
//! messages to the program, nothing parsed beyond the one check that a
//! frame is a frame, until either side hangs up.
//!
//! # Every connection is its own
//!
//! A program may dial any number of times. Each socket is announced
//! under an id of its own, carried by a pair of channels of its own,
//! and answered by a daemon session of its own, so the scopes and
//! channels the program mints inside one connection never meet
//! another's. Nothing is shared between two connections but the begin
//! scope they are announced on.

use std::pin::pin;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use diverge_sdk::shared::containers::daemon;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::sync::oneshot;

use crate::answer::{self, Answer};
use crate::ask;
use crate::own::Own;
use crate::proxy::Proxy;

/// The upgrade, and nothing before it: the connection is served for
/// the container's account, so there is no credential to take and
/// nothing to refuse here.
pub async fn serve(State(proxy): State<Arc<Proxy>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| connection(socket, proxy)).into_response()
}

/// The server opened its half of `connection_id` on `channel`: hand
/// the channel to the connection waiting for it. An id nobody waits
/// on is a channel this end cannot serve, finished with nothing.
pub async fn attach(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32, connection_id: u32) {
    if !proxy.daemons.pair(connection_id, channel) {
        scope.send_channel_response_finish(channel).await;
    }
}

/// One daemon connection, announced and then carried.
///
/// # Announced before anything is read
///
/// The id is registered, the ask goes out, and nothing is read from
/// the socket until the server's half arrives — so a program that
/// writes its first request the instant it connects cannot have it
/// dropped for want of somewhere to put it. The daemon protocol is
/// client-first, so the ask says nothing before that first request is
/// carried; a frame that arrives early all the same is kept for the
/// writer, in order, and written before any other.
///
/// # Two directions, two tasks
///
/// Once paired, a writer task feeds the socket with every frame the
/// ask carries and closes it when the ask ends, and this task reads
/// the program's frames onto the server's half until the socket closes
/// or a frame will not decode, and finishes the half — how the server
/// learns the program hung up. Each direction is ordered on its own
/// and neither waits on the other: the frames of one connection
/// interleave freely with every other connection's on the wire.
///
/// The writer is never awaited: awaiting it would block on a program
/// that stopped reading. It ends on its own when the ask ends, when a
/// write fails, or when this task returns — whichever comes first —
/// and closes the socket as it goes, so a connection either side gave
/// up on is gone without anything waiting for the other.
///
/// # Nothing is read twice, and nothing is parsed
///
/// A frame crosses as the bytes it arrived as. This end decodes each
/// of the program's frames once, and only far enough to know it is one
/// — a frame shorter than its header, of a type the wire does not
/// have, or an `Auth`, which a daemon connection inside a container
/// never carries — and a frame that is none ends the connection rather
/// than being guessed at. What the daemon says is not decoded at all:
/// the program's own wire reads it.
async fn connection(socket: WebSocket, proxy: Arc<Proxy>) {
    let (id, half) = proxy.daemons.announce();
    let Ok((begun, mut channel)) = ask::open(
        &proxy,
        Own::Daemon(daemon::request::Daemon { connection_id: id }),
    )
    .await
    else {
        proxy.daemons.forget(id);
        return;
    };

    // Frames the daemon says before the half is open — the protocol is
    // client-first, so none is expected — are kept for the writer.
    let mut early = Vec::new();
    let mut half = pin!(half);
    let server_channel = loop {
        tokio::select! {
            paired = &mut half => match paired {
                Ok(channel) => break channel,
                Err(_) => {
                    proxy.daemons.forget(id);
                    return;
                }
            },
            next = answer::next(&mut channel) => match next {
                Some(Answer::Frame(bytes)) => early.push(bytes),
                // The ask ended before the half came: the caller
                // declined the connection, or the run is over.
                Some(Answer::Finish) | None => {
                    proxy.daemons.forget(id);
                    return;
                }
            },
        }
    };

    let (mut sink, mut stream) = socket.split();
    // Dropped when this task returns, whichever way: the writer sees
    // the connection is over without being awaited, and closes.
    let (reading, mut read) = oneshot::channel::<()>();
    tokio::spawn(async move {
        for bytes in early {
            if sink.send(Message::Binary(bytes)).await.is_err() {
                return;
            }
        }
        loop {
            tokio::select! {
                next = answer::next(&mut channel) => match next {
                    Some(Answer::Frame(bytes)) => {
                        if sink.send(Message::Binary(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Some(Answer::Finish) | None => break,
                },
                _ = &mut read => break,
            }
        }
        let _ = sink.close().await;
    });

    while let Some(Ok(message)) = stream.next().await {
        let bytes = match message {
            Message::Binary(bytes) => bytes,
            Message::Close(_) => break,
            // A ping, a pong, or text. None of them are this
            // protocol's, and the socket stays open: the wire's own
            // reader ignores them the same way.
            _ => continue,
        };
        if daemon::client::Frame::decode(&bytes).is_err() {
            break;
        }
        begun.scope.send_channel_response(server_channel, &bytes).await;
    }
    begun.scope.send_channel_response_finish(server_channel).await;
    drop(reading);
}
