//! Posting one message, from a scope and the mailboxes.

use std::sync::Arc;

use bytes::Bytes;

use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::provider::endpoints::daemons::message::client::channel_request;
use diverge_sdk::provider::endpoints::daemons::message::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::protocol::mailboxes::{Mailboxes, Outcome, Posted};
use crate::protocol::release::Release;
use crate::protocol::singles::Single;

/// Post `body` from the caller to `to`, and answer its fate. In order:
///
/// 0. [`Unsupported`](response::Frame::Unsupported), then the finish,
///    on a provider that relays no messages. Any connected identity
///    sends past this, announced or not, and listening or not.
/// 1. The caller's window counted — see
///    [`Mailboxes::admit`] — or full, which is
///    [`RateLimited`](response::Frame::RateLimited) with the seconds
///    left, then the finish; nothing is posted. Every send is counted,
///    whoever sends it and whether or not `to` listens.
/// 2. The message posted to `to`'s mailbox — or not, none being open,
///    which is [`NotListening`](response::Frame::NotListening), then
///    the finish. A mailbox is found by its identity alone: `to` need
///    not be announced.
/// 3. Until its fate: the fate, answered as
///    [`Delivered`](response::Frame::Delivered) or `NotListening`, then
///    the finish. A cancel while the message waits takes it out, and
///    the scope finishes with no response before the finish; a cancel
///    after the receiver had it changes nothing, and the fate is
///    waited for still. A channel request that is not a cancel is
///    finished, and the wait goes on. The caller reading no more of
///    the scope, or its connection ending, takes the message out and
///    ends the scope.
/// 4. The withdrawal is a [`Release`], so a scope that panics or is
///    cancelled takes its message out at the drop; the connection's
///    claim on its one message is held with the scope, and goes before
///    its finish either way — see [`Single`].
pub async fn handle(scope: Single<ScopeHandle>, client_identity: &Identity, to: Identity, body: Bytes, mailboxes: Arc<Mailboxes>, capabilities: &Capabilities) {
    if capabilities.messages.is_none() {
        send(&scope, &response::Frame::Unsupported).await;
        scope.send_response_finish().await;
        return;
    }
    if let Err(retry_after) = mailboxes.admit(client_identity).await {
        send(&scope, &response::Frame::RateLimited { retry_after }).await;
        scope.send_response_finish().await;
        return;
    }
    let (mailbox, id, mut fate) = match mailboxes.post(client_identity, &to, body).await {
        Posted::NotListening => {
            send(&scope, &response::Frame::NotListening).await;
            scope.send_response_finish().await;
            return;
        }
        Posted::Queued { mailbox, id, fate } => (mailbox, id, fate),
    };
    let withdrawal = Release::new({
        let mailbox = Arc::clone(&mailbox);
        async move {
            mailbox.withdraw(id).await;
        }
    });
    let answer = loop {
        tokio::select! {
            outcome = &mut fate => break Some(outcome.unwrap_or(Outcome::NotListening)),
            request = scope.recv_channel_request() => {
                let Some(bytes) = request else {
                    break None;
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    Ok(channel_request::Frame::Cancel) => {
                        if mailbox.withdraw(id).await {
                            break None;
                        }
                    }
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            () = scope.responses_gone() => break None,
        }
    };
    withdrawal.now().await;
    match answer {
        Some(Outcome::Delivered) => send(&scope, &response::Frame::Delivered).await,
        Some(Outcome::NotListening) => send(&scope, &response::Frame::NotListening).await,
        None => {}
    }
    scope.send_response_finish().await;
}

/// A message on a connection holding one open already: `AlreadyOpen` and
/// the finish, and nothing else — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time).
pub async fn refuse(scope: ScopeHandle) {
    send(&scope, &response::Frame::AlreadyOpen).await;
    scope.send_response_finish().await;
}

/// One response on the scope.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    frame.encode(&mut Writer::new(&mut buffer)).unwrap_or_else(|never| match never {});
    scope.send_response(&buffer).await;
}
