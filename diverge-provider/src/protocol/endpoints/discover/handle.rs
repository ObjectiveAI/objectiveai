//! Answering a discover listing, from a scope and the discoverable
//! identities, and keeping it.

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::broadcast;

use diverge_sdk::provider::endpoints::discover::client::channel_request;
use diverge_sdk::provider::endpoints::discover::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::protocol::discoverable::Discoverable;
use crate::protocol::singles::Single;

/// List every identity `discoverable` holds — every connected identity
/// whose discover flag was `1` or `2`, with its description, its IP and
/// the address it was dialled at — and keep
/// listing, as an image listing
/// keeps its images. The rules:
///
/// 1. **Subscribed first.** The nudges are subscribed before the
///    snapshot is read, so a connection opening or ending between the
///    two is told after.
/// 2. **The snapshot.** Every identity held, as `Added`, then exactly
///    one `Listed`.
/// 3. **Kept.** On every nudge — and on a lag, which may have skipped
///    some — the identities are read again and what differs is told:
///    each new one as `Added`, then each gone as `Removed`. An identity
///    held still as another item — another description, IP or address:
///    its connection ended and another opened between two reads — is
///    told as `Removed` as it was, then `Added` as it is.
/// 4. **The ends.** The caller's stop, the caller's connection ending
///    (no channel request will come), the caller reading no more of the
///    listing, or the nudges closing end the scope with its finish. A
///    channel request that is not a stop is finished, and the listing
///    goes on.
/// 5. **One per connection.** The connection's claim on its one
///    listing is held with the scope, and goes before the finish — see
///    [`Single`].
///
/// Every connected peer may ask, and the asking identity is listed when
/// it is itself discoverable. Nothing fails: a provider always knows
/// what is connected to it.
pub async fn handle(scope: Single<ScopeHandle>, discoverable: Arc<Discoverable>) {
    let mut nudges = discoverable.subscribe();
    let mut known = discoverable.list().await;
    for listed in known.values() {
        send(&scope, &response::Frame::Added(listed.clone())).await;
    }
    send(&scope, &response::Frame::Listed).await;
    loop {
        tokio::select! {
            nudge = nudges.recv() => {
                match nudge {
                    Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {
                        tell(&scope, &mut known, discoverable.list().await).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            request = scope.recv_channel_request() => {
                let Some(bytes) = request else {
                    break;
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    Ok(channel_request::Frame::Stop) => break,
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            // The caller reads no more of the listing: abandoned, and
            // over as at the stop.
            () = scope.responses_gone() => break,
        }
    }
    scope.send_response_finish().await;
}

/// A discover listing on a connection holding one open already: `AlreadyOpen`
/// and the finish, and nothing else — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time).
pub async fn refuse(scope: ScopeHandle) {
    send(&scope, &response::Frame::AlreadyOpen).await;
    scope.send_response_finish().await;
}

/// What differs between `known` and `now`, told — every identity new
/// or listed anew as added, after every identity gone or listed anew as
/// removed — and `now` kept as what is known (rule 3).
async fn tell(scope: &ScopeHandle, known: &mut BTreeMap<Identity, response::Discoverable>, now: BTreeMap<Identity, response::Discoverable>) {
    for (identity, listed) in known.iter() {
        if now.get(identity) != Some(listed) {
            send(scope, &response::Frame::Removed(listed.clone())).await;
        }
    }
    for (identity, listed) in &now {
        if known.get(identity) != Some(listed) {
            send(scope, &response::Frame::Added(listed.clone())).await;
        }
    }
    *known = now;
}

/// One response, encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
