//! The loop every list kept open runs.

use diverge_sdk::wire::encode::Encode;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::sync::broadcast;

use super::{Change, Source, diff};
use crate::daemon::{Daemon, Kind};
use crate::serve::{files, reply};
use crate::store;

/// Send the listing and keep it, until the client cancels or is gone.
///
/// The word is subscribed to before the source is read, so a change
/// between the reading and the hearing is in one or the other. The
/// first `count` items read are sent as `Added`, then `Listed`. Then,
/// at every word naming one of `kinds`, and whenever the listing fell
/// behind the words, the source is read again, the first `count`
/// kept as the listing, and the difference against what was last
/// sent is told: `Added`, `Changed`, `Removed`. The client's cancel
/// ends it, as does the client's connection ending; a store that
/// could not be read is the error, for the handler to send.
pub async fn listing<S, F, Fr>(scope: &ScopeHandle, daemon: &Daemon, kinds: &[Kind], count: Option<u64>, source: &S, frame: F) -> Result<(), store::Error>
where
    S: Source,
    F: Fn(Change<S::Item>) -> Fr,
    Fr: Encode,
{
    let cap = count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let mut words = daemon.live.changes();
    let mut known = source.read().await?;
    known.truncate(cap);
    for (_, item) in &known {
        reply::reply(scope, &frame(Change::Added(item.clone()))).await;
    }
    reply::reply(scope, &frame(Change::Listed)).await;
    loop {
        tokio::select! {
            word = words.recv() => {
                let again = match word {
                    Ok(kind) => kinds.contains(&kind),
                    Err(broadcast::error::RecvError::Lagged(_)) => true,
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
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
            () = files::cancelled(scope) => return Ok(()),
        }
    }
}
