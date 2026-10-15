//! The loop every list kept open runs.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::capacity::server::response::Frame as Capacity;
use diverge_sdk::wire::encode::Encode;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::sync::broadcast;

use super::{Change, Patch, Source, Unpatched, diff};
use crate::daemon::{CapacityChange, Daemon, Kind};
use crate::serve::{files, reply};

/// Send the listing and keep it, until the client cancels or is gone.
///
/// The word is subscribed to before the source is read, so a change
/// between the reading and the hearing is in one or the other. The
/// first `count` items read are sent as `Added`, then `Listed`. Then,
/// at every word naming one of `kinds`, and whenever the listing fell
/// behind the words, the source is read again, the first `count`
/// kept as the listing, and the difference against what was last
/// sent is told: `Added`, `Changed`, `Removed`. The client's cancel
/// ends it, as does the client's connection ending; a source that
/// could not be read is its error, for the handler to send.
pub async fn listing<S, F, Fr>(scope: &ScopeHandle, daemon: &Daemon, kinds: &[Kind], count: Option<u64>, source: &S, frame: F) -> Result<(), S::Error>
where
    S: Source,
    F: Fn(Change<S::Item>) -> Fr,
    Fr: Encode,
{
    kept(scope, daemon, kinds, count, source, frame, &Unpatched).await
}

/// [`listing`], and every capacity swap heard besides: one naming a
/// provider the listing lists is taken through `patch` — the known
/// item patched in place, the frame it answers sent — and the records
/// are not read for it. A listing that fell behind the swaps reads
/// everything again, as it does behind the words.
pub async fn listing_patched<S, F, Fr, P>(scope: &ScopeHandle, daemon: &Daemon, kinds: &[Kind], count: Option<u64>, source: &S, frame: F, patch: &P) -> Result<(), S::Error>
where
    S: Source,
    F: Fn(Change<S::Item>) -> Fr,
    Fr: Encode,
    P: Patch<S::Key, S::Item, Fr>,
{
    kept(scope, daemon, kinds, count, source, frame, patch).await
}

/// The one loop under both: subscribed to the words and the swaps
/// before the first read, so nothing falls between.
async fn kept<S, F, Fr, P>(scope: &ScopeHandle, daemon: &Daemon, kinds: &[Kind], count: Option<u64>, source: &S, frame: F, patch: &P) -> Result<(), S::Error>
where
    S: Source,
    F: Fn(Change<S::Item>) -> Fr,
    Fr: Encode,
    P: Patch<S::Key, S::Item, Fr>,
{
    let cap = count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let mut words = daemon.live.changes();
    let mut swaps = daemon.live.capacity_changes();
    let mut known = source.read().await?;
    known.truncate(cap);
    for (_, item) in &known {
        reply::reply(scope, &frame(Change::Added(item.clone()))).await;
    }
    reply::reply(scope, &frame(Change::Listed)).await;
    loop {
        let again = tokio::select! {
            word = words.recv() => match word {
                Ok(kind) => kinds.contains(&kind),
                Err(broadcast::error::RecvError::Lagged(_)) => true,
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            },
            swap = swaps.recv() => match swap {
                Ok(CapacityChange { identity, capacity }) => {
                    patched(scope, &mut known, patch, &identity, &capacity).await;
                    false
                }
                Err(broadcast::error::RecvError::Lagged(_)) => true,
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            },
            () = files::cancelled(scope) => return Ok(()),
        };
        if again {
            let mut now = source.read().await?;
            now.truncate(cap);
            for change in diff(&known, &now) {
                reply::reply(scope, &frame(change)).await;
            }
            known = now;
        }
    }
}

/// One swap taken: the item listed under the provider's key, if any,
/// patched in place, and the frame the patch answers sent.
async fn patched<K, I, Fr, P>(scope: &ScopeHandle, known: &mut [(K, I)], patch: &P, identity: &Identity, capacity: &Capacity)
where
    K: PartialEq,
    Fr: Encode,
    P: Patch<K, I, Fr>,
{
    let Some(key) = patch.key(identity) else {
        return;
    };
    let Some((_, item)) = known.iter_mut().find(|(known, _)| *known == key) else {
        return;
    };
    if let Some(frame) = patch.apply(item, capacity) {
        reply::reply(scope, &frame).await;
    }
}
