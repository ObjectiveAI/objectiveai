//! Relaying one daemon connection, from a scope and the acceptors.

use diverge_sdk::shared::Identity;
use std::net::IpAddr;
use std::sync::Arc;

use tokio::sync::oneshot;

use diverge_sdk::provider::endpoints::daemons::connect::client::request;
use diverge_sdk::provider::endpoints::daemons::connect::server::{channel_request, response};
use crate::protocol::endpoints::containers::run::encoded::encoded;
use diverge_sdk::provider::endpoints::daemons::accept::server::channel_request as announce;
use crate::protocol::acceptors::{Accepting, Acceptors};
use crate::protocol::release::Release;
use diverge_sdk::shared::daemons::Connection;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::server::answer::{Answer, answer};
use diverge_sdk::wire::server::channel::Channel;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Connect the caller to the daemon it named, if that daemon accepts
/// here and takes the connection, and relay the connection until
/// either end hangs up.
///
/// In order:
///
/// 1. The acceptor, found among those accepting by the identity the
///    request names — or not, which is the scope's one `Error`,
///    `{"kind":"missing"}`, then the finish.
/// 2. The provider's half opened on the accept scope: an id minted
///    for the connection, and a channel carrying the connector's
///    identity and address — this connection's, the provider's own
///    word — and the mode as the request asserted it. Then the wait:
///    the acceptor's half, a channel it opens quoting the id, is the
///    connection taken; the provider's half finished before that is
///    the acceptor declining, `{"kind":"denied"}`, then the finish;
///    the accept scope ending first is `{"kind":"missing"}`; the
///    connector reading no more of this scope ends the wait the same
///    way. The id waits among the acceptor's connections from the
///    mint to the end of the wait, and no longer, whichever way the
///    wait ends — a [`Release`] forgets it, run in place or at the
///    drop.
/// 3. Exactly one response, `Connected`, and the one channel opened
///    on the scope, where the connector's frames go.
/// 4. The relay, both ways: every channel response the connector
///    sends on that channel is a channel response on the acceptor's
///    half, byte for byte; every channel response the acceptor sends
///    on the provider's half is a response on this scope, byte for
///    byte, behind the tag. The connector finishing its channel, or
///    going, finishes the acceptor's half; the acceptor finishing the
///    provider's half, its accept scope ending, or the connector
///    reading no more of this scope, ends the scope.
/// 5. The finish.
///
/// The provider's half, given up anywhere above, is dropped: the drop
/// tells the acceptor its answer is abandoned, and the acceptor's
/// finish of it, which an abandoned answer still sends, is what gives
/// its number back on the accept scope.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle(scope: ScopeHandle, request: request::Frame, client_identity: &Identity, address: IpAddr, acceptors: Arc<Acceptors>) {
    let scope = Arc::new(scope);
    let Some(accepting) = acceptors.lookup(&request.daemon).await else {
        refuse(&scope, missing()).await;
        return;
    };
    let (connection_id, half) = accepting.open().await;
    let waiting = Release::new({
        let accepting = Arc::clone(&accepting);
        async move { accepting.forget(connection_id).await }
    });
    let announced = announce::Frame(Connection {
        connection_id,
        identity: client_identity.clone(),
        address,
    });
    let Some(payload) = encoded(&announced) else {
        waiting.now().await;
        refuse(&scope, missing()).await;
        return;
    };
    let mut theirs = accepting.scope.send_channel_request(&payload).await;
    let daemon_half = taken(&scope, &accepting, half, &mut theirs).await;
    // Delivered or not, the id waits no more: a half delivered took it
    // out already, and this is then nothing.
    waiting.now().await;
    let Some(daemon_half) = daemon_half else {
        refuse(&scope, denied()).await;
        return;
    };
    send(&scope, response::Frame::Connected).await;
    let Some(frames) = encoded(&channel_request::Frame) else {
        accepting.scope.send_channel_response_finish(daemon_half).await;
        scope.send_response_finish().await;
        return;
    };
    let mut ours = scope.send_channel_request(&frames).await;
    let to_acceptor = {
        let accepting = Arc::clone(&accepting);
        tokio::spawn(async move {
            while let Some(bytes) = ours.response_receiver.recv().await {
                match answer(&bytes) {
                    Some(Answer::Frame(payload)) => accepting.scope.send_channel_response(daemon_half, &payload).await,
                    Some(Answer::Finish) => break,
                    None => {}
                }
            }
            accepting.scope.send_channel_response_finish(daemon_half).await;
        })
    };
    let mut ended = accepting.ended();
    loop {
        tokio::select! {
            received = theirs.response_receiver.recv() => match received {
                Some(bytes) => match answer(&bytes) {
                    Some(Answer::Frame(payload)) => send(&scope, response::Frame::Frame(&payload)).await,
                    Some(Answer::Finish) => break,
                    None => {}
                },
                None => break,
            },
            _ = ended.changed() => break,
            () = scope.responses_gone() => break,
        }
    }
    to_acceptor.abort();
    accepting.scope.send_channel_response_finish(daemon_half).await;
    scope.send_response_finish().await;
}

/// Wait for the acceptor to take the connection: its half's channel
/// number, or `None` when the provider's half was finished first, the
/// acceptor's connection went, its accept scope ended, or the
/// connector on `scope` reads no more of it.
async fn taken(scope: &ScopeHandle, accepting: &Accepting, half: oneshot::Receiver<u32>, theirs: &mut Channel) -> Option<u32> {
    let mut ended = accepting.ended();
    let mut half = std::pin::pin!(half);
    loop {
        if *ended.borrow_and_update() {
            return None;
        }
        tokio::select! {
            channel = half.as_mut() => return channel.ok(),
            answered = theirs.response_receiver.recv() => match answered {
                // The acceptor's session says nothing before it is
                // asked, so a frame here is read past; its finish is
                // the decline.
                Some(bytes) => match answer(&bytes) {
                    Some(Answer::Finish) => return None,
                    _ => continue,
                },
                None => return None,
            },
            _ = ended.changed() => return None,
            () = scope.responses_gone() => return None,
        }
    }
}

/// One response on the scope.
async fn send(scope: &ScopeHandle, frame: response::Frame<'_>) {
    if let Some(payload) = encoded(&frame) {
        scope.send_response(&payload).await;
    }
}

/// The error, then the finish.
async fn refuse(scope: &ScopeHandle, error: Error) {
    send(scope, response::Frame::Error(error)).await;
    scope.send_response_finish().await;
}

/// No daemon accepts under that identity here.
fn missing() -> Error {
    Error(serde_json::json!({ "kind": "missing" }))
}

/// The acceptor declined.
fn denied() -> Error {
    Error(serde_json::json!({ "kind": "denied" }))
}
