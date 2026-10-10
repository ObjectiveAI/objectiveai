//! Relaying one daemon connection, from a scope and the acceptors.

use std::net::IpAddr;
use std::sync::Arc;

use futures_util::future::{self, Either};
use tokio::sync::oneshot;

use diverge_sdk::provider::endpoints::daemons::connect::client::request;
use diverge_sdk::provider::endpoints::daemons::connect::server::{channel_request, response};
use crate::protocol::endpoints::containers::run::encoded::encoded;
use diverge_sdk::provider::endpoints::daemons::accept::server::channel_request as announce;
use crate::protocol::acceptors::{Accepting, Acceptors};
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
///    the accept scope ending first is `{"kind":"missing"}`.
/// 3. Exactly one response, `Connected`, and the one channel opened
///    on the scope, where the connector's frames go.
/// 4. The relay, both ways: every channel response the connector
///    sends on that channel is a channel response on the acceptor's
///    half, byte for byte; every channel response the acceptor sends
///    on the provider's half is a response on this scope, byte for
///    byte, behind the tag. The connector finishing its channel, or
///    going, finishes the acceptor's half; the acceptor finishing the
///    provider's half, or its accept scope ending, ends the scope.
/// 5. The finish.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, and hands the result here.
pub async fn handle(scope: ScopeHandle, request: request::Frame, client_identity: &str, address: IpAddr, acceptors: Arc<Acceptors>) {
    let scope = Arc::new(scope);
    let Some(accepting) = acceptors.lookup(&request.daemon).await else {
        refuse(&scope, missing()).await;
        return;
    };
    let (connection_id, half) = accepting.open().await;
    let announced = announce::Frame(Connection {
        connection_id,
        identity: client_identity.to_string(),
        address,
        mode: request.mode,
    });
    let Some(payload) = encoded(&announced) else {
        accepting.forget(connection_id).await;
        refuse(&scope, missing()).await;
        return;
    };
    let mut theirs = accepting.scope.send_channel_request(&payload).await;
    let Some(daemon_half) = taken(&accepting, half, &mut theirs).await else {
        accepting.forget(connection_id).await;
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
        let next = std::pin::pin!(theirs.response_receiver.recv());
        let over = std::pin::pin!(ended.changed());
        match future::select(next, over).await {
            Either::Left((Some(bytes), _)) => match answer(&bytes) {
                Some(Answer::Frame(payload)) => send(&scope, response::Frame::Frame(&payload)).await,
                Some(Answer::Finish) => break,
                None => {}
            },
            Either::Left((None, _)) => break,
            Either::Right(_) => break,
        }
    }
    to_acceptor.abort();
    accepting.scope.send_channel_response_finish(daemon_half).await;
    scope.send_response_finish().await;
}

/// Wait for the acceptor to take the connection: its half's channel
/// number, or `None` when the provider's half was finished first, the
/// acceptor's connection went, or its accept scope ended.
async fn taken(accepting: &Accepting, half: oneshot::Receiver<u32>, theirs: &mut Channel) -> Option<u32> {
    let mut ended = accepting.ended();
    let mut half = std::pin::pin!(half);
    loop {
        if *ended.borrow_and_update() {
            return None;
        }
        let answered = std::pin::pin!(theirs.response_receiver.recv());
        let over = std::pin::pin!(ended.changed());
        match future::select(half.as_mut(), future::select(answered, over)).await {
            Either::Left((Ok(channel), _)) => return Some(channel),
            Either::Left((Err(_), _)) => return None,
            Either::Right((Either::Left((Some(bytes), _)), _)) => match answer(&bytes) {
                // The acceptor's session says nothing before it is
                // asked, so a frame here is read past; its finish
                // is the decline.
                Some(Answer::Finish) => return None,
                _ => continue,
            },
            Either::Right((Either::Left((None, _)), _)) => return None,
            Either::Right((Either::Right(_), _)) => return None,
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
