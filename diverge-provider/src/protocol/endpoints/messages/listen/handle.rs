//! Keeping a mailbox open, from a scope and the mailboxes.

use std::sync::Arc;

use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::provider::endpoints::messages::listen::client::channel_request;
use diverge_sdk::provider::endpoints::messages::listen::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::protocol::discoverable::Discoverable;
use crate::protocol::mailboxes::{Mailboxes, Outcome};
use crate::protocol::release::Release;
use crate::protocol::singles::Single;

/// Open the caller's mailbox, and hand the caller every message sent
/// to it, one at a time, until the caller cancels. In order:
///
/// 0. [`Unsupported`](response::Frame::Unsupported), then the finish,
///    on a provider that relays no messages; then
///    [`NotDiscoverable`](response::Frame::NotDiscoverable), then the
///    finish, for a caller whose connection is not in discovery.
/// 1. The mailbox, opened under the connection's identity — or not,
///    one being open under it on another connection, which is
///    [`AlreadyOpen`](response::Frame::AlreadyOpen), then the finish.
///    The connection's own claim on its one listen was made before
///    this was called, and refused there as the same answer.
/// 2. For the scope's life, the oldest message waiting, sent as
///    [`Message`](response::Frame::Message); then the scope's room
///    waited for — the caller's reader taking it — and its sender
///    answered [`Delivered`](Outcome::Delivered), or
///    [`NotListening`](Outcome::NotListening) when the caller reads no
///    more of the scope first, which ends it. One message is sent at a
///    time, so what the provider has sent and the caller has not taken
///    is at most one.
/// 3. A cancel, read before the next message, ends the scope; so does
///    the caller reading no more of the scope, or its connection
///    ending. A channel request that is not a cancel is finished, and
///    the listen goes on.
/// 4. The mailbox closed — every message still queued answered not
///    listening — then the connection's claim, and the finish. The
///    closing is a [`Release`], so a scope that panics or is cancelled
///    closes it at the drop; the claim is held with the scope, and goes
///    before its finish either way — see [`Single`].
pub async fn handle(scope: Single<ScopeHandle>, client_identity: &Identity, mailboxes: Arc<Mailboxes>, discoverable: Arc<Discoverable>, capabilities: &Capabilities) {
    if capabilities.messages.is_none() {
        send(&scope, &response::Frame::Unsupported).await;
        scope.send_response_finish().await;
        return;
    }
    if !discoverable.is_listed(client_identity).await {
        send(&scope, &response::Frame::NotDiscoverable).await;
        scope.send_response_finish().await;
        return;
    }
    let Some(mailbox) = mailboxes.open(client_identity).await else {
        send(&scope, &response::Frame::AlreadyOpen).await;
        scope.send_response_finish().await;
        return;
    };
    let closing = Release::new({
        let mailboxes = Arc::clone(&mailboxes);
        let identity = client_identity.clone();
        async move { mailboxes.close(&identity).await }
    });
    loop {
        let waiting = tokio::select! {
            // A cancel waiting is read before the next message (rule 3).
            biased;
            request = scope.recv_channel_request() => {
                let Some(bytes) = request else {
                    break;
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    Ok(channel_request::Frame::Cancel) => break,
                    Err(_) => {
                        scope.send_channel_response_finish(channel).await;
                        continue;
                    }
                }
            }
            () = scope.responses_gone() => break,
            waiting = mailbox.next() => waiting,
        };
        let mut buffer = Vec::new();
        if response::Frame::encode_message(&waiting.from, &waiting.body, &mut Writer::new(&mut buffer)).is_err() {
            waiting.answer(Outcome::NotListening);
            continue;
        }
        scope.send_response(&buffer).await;
        drop(buffer);
        if scope.response_taken().await {
            waiting.answer(Outcome::Delivered);
        } else {
            waiting.answer(Outcome::NotListening);
            break;
        }
    }
    closing.now().await;
    scope.send_response_finish().await;
}

/// A listen on a connection holding one open already: `AlreadyOpen`
/// and the finish, and nothing else — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time).
pub async fn refuse(scope: ScopeHandle) {
    send(&scope, &response::Frame::AlreadyOpen).await;
    scope.send_response_finish().await;
}

/// One response on the scope, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
