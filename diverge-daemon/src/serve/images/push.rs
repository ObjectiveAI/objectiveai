//! Pushing an image into the daemon's Store on a provider.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::images::push::client::request;
use diverge_sdk::daemon::endpoints::images::push::server::response::{Blob, Exists, Frame, Pushed};
use diverge_sdk::daemon::grant::images::Action;
use diverge_sdk::provider::endpoints::images::push::client::request::Frame as Ask;
use diverge_sdk::provider::endpoints::images::push::server::response::Frame as Said;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::credit::Receiver;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode as _, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::frame::server::ServerFrame;
use diverge_sdk::wire::server::channel::Channel;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::task::JoinSet;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;
use crate::volumes;

/// Answer the push — each blob pulled as it comes, then how it ended —
/// and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let scope = Arc::new(scope);
    let answer = match serve(&scope, frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The answer that ends the push, every blob pulled sent on the scope
/// before it. The rules:
///
/// 1. `Forbidden` with no `push` grant over images at all.
/// 2. `NotFound` for a provider the daemon keeps no record of;
///    `Forbidden` for one no `push` grant reaches, by its identity or
///    its tags.
/// 3. Then the provider is asked, as the daemon itself, so the image
///    goes into the daemon's Store there. `Error` for a provider not
///    connected now, or a push that would not open.
/// 4. **The provider's asks are the client's.** Every channel the
///    provider opens on its push is carried to the client and back by
///    [`carry`], for as long as the push runs; the daemon answers none
///    of them itself.
/// 5. **The provider's answers, relayed.** Each blob pulled is relayed
///    as it comes. The provider's `Pushed`, `Exists` and its five
///    refusals are relayed as they are; its error is the `Error`, in
///    its words; an exchange that broke, or ended with no result, is
///    the `Error`, in the daemon's.
/// 6. **The carrying ends with the push**: whatever answer of the
///    client's is still on its way then is let go.
async fn serve(scope: &Arc<ScopeHandle>, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    drop(conn);
    // Rule 1.
    if !judge::images::holds(&standing, Action::Push) {
        return Ok(Frame::Forbidden);
    }
    // Rule 2.
    let records = daemon.providers.all();
    let Some(record) = records.iter().find(|record| record.identity == frame.provider) else {
        return Ok(Frame::NotFound);
    };
    if !judge::images::over(&standing, Action::Push, record) {
        return Ok(Frame::Forbidden);
    }
    scope.allowed().await;
    // Rule 3.
    let handle = match volumes::handle(daemon, &frame.provider).await {
        Ok(handle) => handle,
        Err(fail) => return Ok(Frame::Error(fail.error())),
    };
    let ask = Ask {
        image: frame.image,
        description: frame.description,
    };
    let mut payload = Vec::new();
    if let Err(error) = ask.encode(&mut Writer::new(&mut payload)) {
        return Ok(Frame::Error(reply::failure(&error)));
    }
    let mut remote = match handle.send_request(&payload).await {
        Ok(remote) => remote,
        Err(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    // Rule 4.
    let carrying = tokio::spawn(carry(Arc::clone(scope), handle.clone(), remote.scope, remote.request_receiver));
    // Rule 5.
    let answer = loop {
        let Some(bytes) = remote.response_receiver.recv().await else {
            break Frame::Error(reply::failure(&"the connection to the provider ended with the push open"));
        };
        let payload = match ServerFrame::decode(&bytes) {
            Ok(ServerFrame::Response { payload, .. }) => payload,
            Ok(ServerFrame::ResponseFinish { .. }) => break Frame::Error(reply::failure(&"the provider ended the push with no result")),
            Ok(_) => continue,
            Err(error) => break Frame::Error(reply::failure(&error)),
        };
        match Said::decode(payload) {
            Ok(Said::Blob(blob)) => {
                reply::reply(
                    scope,
                    &Frame::Blob(Blob {
                        digest: blob.digest,
                        bytes: blob.bytes,
                    }),
                )
                .await;
            }
            Ok(Said::Pushed(pushed)) => {
                break Frame::Pushed(Pushed {
                    digest: pushed.digest,
                    bytes: pushed.bytes,
                    source: pushed.source,
                });
            }
            Ok(Said::Exists(exists)) => break Frame::Exists(Exists { digest: exists.digest }),
            Ok(Said::InsufficientCapacity) => break Frame::InsufficientCapacity,
            Ok(Said::Unavailable) => break Frame::Unavailable,
            Ok(Said::ProofFailed) => break Frame::ProofFailed,
            Ok(Said::Malformed) => break Frame::Malformed,
            Ok(Said::Unsupported) => break Frame::Unsupported,
            Ok(Said::Error(error)) => break Frame::Error(error),
            Err(error) => break Frame::Error(reply::failure(&error)),
        }
    };
    // Rule 6.
    carrying.abort();
    Ok(answer)
}

/// Every ask the provider opens on its push, carried to the client and
/// its answer carried back, until the provider's scope stops yielding
/// asks. The rules:
///
/// 1. **As it came.** Each ask is opened on the client's scope with
///    the provider's own payload, unread, and each answer is one task
///    of its own: several are carried at once.
/// 2. **Under the provider's number.** Each side minted its own channel
///    numbers; the pair lives in the task that carries that answer.
/// 3. **Ends with its owner.** Let go, this lets go of every answer it
///    is still carrying.
async fn carry(scope: Arc<ScopeHandle>, handle: Handle, remote: u32, mut asks: Receiver) {
    let mut answers = JoinSet::new();
    loop {
        let bytes = tokio::select! {
            received = asks.recv() => match received {
                Some(bytes) => bytes,
                None => break,
            },
            Some(_) = answers.join_next(), if !answers.is_empty() => continue,
        };
        let Ok(ServerFrame::ChannelRequest { channel, payload, .. }) = ServerFrame::decode(&bytes) else {
            continue;
        };
        let opened = scope.send_channel_request(payload).await;
        answers.spawn(answer(handle.clone(), remote, channel, opened));
    }
    while answers.join_next().await.is_some() {}
}

/// The client's answer on one channel, sent to the provider under the
/// provider's number, to its end. The rules:
///
/// 1. **As it came.** Every frame of the answer is sent on as the
///    client wrote it.
/// 2. **Always finished.** The client's finish is the provider's
///    finish. So is the answer ending without one — a client that
///    reads no asks, that went away, or whose scope ended: the
///    provider is sent the finish all the same, which after nothing is
///    the wire's "not held", and after part of a blob is a blob whose
///    bytes do not hash to its digest. A provider is never left
///    waiting on an ask nobody will answer.
/// 3. The channel is held here for as long as it is carried, since its
///    dropping is its closing on the client's side.
async fn answer(handle: Handle, remote: u32, channel: u32, mut opened: Channel) {
    while let Some(bytes) = opened.response_receiver.recv().await {
        match ClientFrame::decode(&bytes) {
            Ok(ClientFrame::ChannelResponse { payload, .. }) => {
                if handle.send_channel_response(remote, channel, payload).await.is_err() {
                    return;
                }
            }
            Ok(ClientFrame::ChannelResponseFinish { .. }) => break,
            _ => {}
        }
    }
    let _ = handle.send_channel_response_finish(remote, channel).await;
}
