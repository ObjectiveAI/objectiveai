//! Answering a daemons list, from a scope and the announced
//! identities, and keeping it.

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::broadcast;

use diverge_sdk::provider::endpoints::daemons::list::client::channel_request;
use diverge_sdk::provider::endpoints::daemons::list::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::protocol::listables::Listables;
use crate::protocol::singles::Single;

/// List every identity `listables` holds — every connected identity
/// that holds a `daemons::announce` open, for exactly as long as it
/// holds it, with its announcement, its IP and the address it was
/// dialled at — and keep listing, as an image listing keeps its
/// images. The rules:
///
/// 1. **Subscribed first.** The nudges are subscribed before the
///    snapshot is read, so an announce opening or ending between the
///    two is told after.
/// 2. **The snapshot.** Every identity held, as `Added`, then exactly
///    one `Listed`.
/// 3. **Kept.** On every nudge — and on a lag, which may have skipped
///    some — the identities are read again and what differs is told:
///    each gone as `Removed`, then each new one as `Added`. An identity
///    held still as another item — another announcement, IP or address:
///    its announce ended and another opened between two reads, on the
///    same connection or on a new one — is told as `Removed` as it was,
///    then `Added` as it is.
/// 3a. **Listening, by the key.** An identity held still as the same
///    item but for `listening` is told `Listening` or `NotListening`,
///    the identity alone, after the removals and before the additions.
/// 4. **The ends.** The caller's cancel, the caller's connection ending
///    (no channel request will come), the caller reading no more of the
///    listing, or the nudges closing end the scope with its finish. A
///    channel request that is not a cancel is finished, and the listing
///    goes on.
/// 5. **One per connection.** The connection's claim on its one
///    listing is held with the scope, and goes before the finish — see
///    [`Single`].
///
/// Every connected peer may ask, announced or not, and the asking
/// identity is listed when it is itself announced. Nothing fails: a
/// provider always knows what is announced to it.
pub async fn handle(scope: Single<ScopeHandle>, listables: Arc<Listables>) {
    let mut nudges = listables.subscribe();
    let mut known = listables.list().await;
    for listed in known.values() {
        send(&scope, &response::Frame::Added(listed.clone())).await;
    }
    send(&scope, &response::Frame::Listed).await;
    loop {
        tokio::select! {
            nudge = nudges.recv() => {
                match nudge {
                    Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {
                        tell(&scope, &mut known, listables.list().await).await;
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
                    Ok(channel_request::Frame::Cancel) => break,
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            // The caller reads no more of the listing: abandoned, and
            // over as at the cancel.
            () = scope.responses_gone() => break,
        }
    }
    scope.send_response_finish().await;
}

/// A daemons list on a connection holding one open already: `AlreadyOpen`
/// and the finish, and nothing else — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time).
pub async fn refuse(scope: ScopeHandle) {
    send(&scope, &response::Frame::AlreadyOpen).await;
    scope.send_response_finish().await;
}

/// What differs between `known` and `now`, told — every identity new
/// or listed anew as added, after every identity gone or listed anew as
/// removed — and `now` kept as what is known (rule 3).
async fn tell(scope: &ScopeHandle, known: &mut BTreeMap<Identity, response::Listed>, now: BTreeMap<Identity, response::Listed>) {
    // The same announce's item, but for `listening` (rule 3a).
    let same = |was: &response::Listed, is: &response::Listed| {
        was.announcement == is.announcement && was.ip == is.ip && was.address == is.address
    };
    for (identity, listed) in known.iter() {
        if !now.get(identity).is_some_and(|is| same(listed, is)) {
            send(scope, &response::Frame::Removed(listed.clone())).await;
        }
    }
    for (identity, listed) in known.iter() {
        if let Some(is) = now.get(identity)
            && same(listed, is)
            && listed.listening != is.listening
        {
            let frame = if is.listening { response::Frame::Listening(identity.clone()) } else { response::Frame::NotListening(identity.clone()) };
            send(scope, &frame).await;
        }
    }
    for (identity, listed) in &now {
        if !known.get(identity).is_some_and(|was| same(was, listed)) {
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
