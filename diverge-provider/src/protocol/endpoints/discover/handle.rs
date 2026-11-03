//! Answering a discover listing, from a scope and the discoverable
//! identities, and keeping it.

use std::collections::BTreeSet;
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

/// List every identity `discoverable` holds — every connected identity
/// whose discover flag was `1` — and keep listing, as an image listing
/// keeps its images. The rules:
///
/// 1. **Subscribed first.** The nudges are subscribed before the
///    snapshot is read, so a connection opening or ending between the
///    two is told after.
/// 2. **The snapshot.** Every identity held, as `Added`, then exactly
///    one `Listed`.
/// 3. **Kept.** On every nudge — and on a lag, which may have skipped
///    some — the identities are read again and what differs is told:
///    each new one as `Added`, then each gone as `Removed`.
/// 4. **The ends.** The caller's stop, the caller's connection ending
///    (no channel request will come), or the nudges closing end the
///    scope with its finish. A channel request that is not a stop is
///    finished, and the listing goes on.
///
/// Every connected peer may ask, and the asking identity is listed when
/// it is itself discoverable. Nothing fails: a provider always knows
/// what is connected to it.
pub async fn handle(scope: ScopeHandle, discoverable: Arc<Discoverable>) {
    let mut nudges = discoverable.subscribe();
    let mut known = discoverable.list().await;
    for identity in &known {
        send(&scope, &response::Frame::Added(identity.clone())).await;
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
        }
    }
    scope.send_response_finish().await;
}

/// What differs between `known` and `now`, told — every identity new
/// as added, then every identity gone as removed — and `now` kept as
/// what is known.
async fn tell(scope: &ScopeHandle, known: &mut BTreeSet<Identity>, now: BTreeSet<Identity>) {
    for identity in now.difference(known) {
        send(scope, &response::Frame::Added(identity.clone())).await;
    }
    for identity in known.difference(&now) {
        send(scope, &response::Frame::Removed(identity.clone())).await;
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
