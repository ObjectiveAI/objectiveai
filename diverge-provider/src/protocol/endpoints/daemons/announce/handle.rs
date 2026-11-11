//! Keeping the caller listed, from a scope, the mailboxes and the
//! listables.

use std::net::IpAddr;
use std::sync::Arc;

use diverge_sdk::provider::endpoints::daemons::announce::client::channel_request;
use diverge_sdk::provider::endpoints::daemons::announce::client::request;
use diverge_sdk::provider::endpoints::daemons::announce::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::protocol::listables::Listables;
use crate::protocol::mailboxes::Mailboxes;
use crate::protocol::release::Release;
use crate::protocol::singles::Single;

/// List the caller in the provider's `daemons::list`, with the
/// request's announcement, until the caller cancels. In order:
///
/// 1. The connection's own claim on its one announce was made before
///    this was called, and refused there as
///    [`AlreadyOpen`](response::Frame::AlreadyOpen), then the finish —
///    see [`refuse`]. The announce already open is not changed by it.
/// 2. The caller entered in the listables under the connection's
///    identity, with the request's announcement, the connection's `ip`
///    — the socket's other end — and the address `dialled` when this
///    provider dialled the connection. It is entered through the
///    [`Mailboxes`], so it is listed listening at once when its
///    `daemons::listen` is open already — see
///    [`Mailboxes::announce`].
/// 3. No response is sent: the scope open is the whole answer.
/// 4. For the scope's life, nothing but the wait for its end: the
///    caller's cancel, read before anything else; the caller's
///    connection ending, when no channel request will come; or the
///    caller reading no more of the scope. A channel request that is
///    not a cancel is finished, and the announce goes on.
/// 5. The caller let go from the listables, then the connection's
///    claim, and the finish. The leaving is a [`Release`], so a scope
///    that panics or is cancelled leaves at the drop; the claim is held
///    with the scope, and goes before its finish either way — see
///    [`Single`].
///
/// Nothing fails: every provider serves the announce, and a request
/// that decoded is a well-formed announcement. Announcing asks nothing
/// of listening, of sending or of accepting, and none of them asks it.
pub async fn handle(
    scope: Single<ScopeHandle>,
    frame: request::Frame,
    client_identity: &Identity,
    ip: IpAddr,
    dialled: Option<String>,
    mailboxes: Arc<Mailboxes>,
    listables: Arc<Listables>,
) {
    mailboxes.announce(client_identity, frame.announcement, ip, dialled).await;
    let leaving = Release::new({
        let identity = client_identity.clone();
        async move { listables.leave(&identity).await }
    });
    loop {
        tokio::select! {
            // A cancel waiting is read before anything else (rule 4).
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
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            // The caller reads no more of the scope: abandoned, and
            // over as at the cancel.
            () = scope.responses_gone() => break,
        }
    }
    leaving.now().await;
    scope.send_response_finish().await;
}

/// An announce on a connection holding one open already: `AlreadyOpen`
/// and the finish, and nothing else — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time).
pub async fn refuse(scope: ScopeHandle) {
    let mut buffer = Vec::new();
    response::Frame::AlreadyOpen.encode(&mut Writer::new(&mut buffer)).unwrap_or_else(|never| match never {});
    scope.send_response(&buffer).await;
    scope.send_response_finish().await;
}
