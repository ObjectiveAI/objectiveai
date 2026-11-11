//! Listing peers through the connected providers, and keeping the
//! listing.

use std::future::Future;
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::providers::daemons::list::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::list::server::response::{Listed, Frame};
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::serve::stream::{self, Source};
use crate::store;

/// Send the listing and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. **Judged.** One `Forbidden` with no `providers_daemons`
///    grant holding `list` over any provider.
/// 2. **Watching.** The scope is a watcher of the daemon's
///    [`Listings`](crate::listings::Listings) from before its first
///    read to its end, however it ends: every connected provider's
///    listing open and whole before anything is sent.
/// 3. **The listing.** Every identity the merge holds, in order, the
///    first `count` of them, one frame each, then the word that the
///    listing is whole — and from then on each identity added, changed
///    or removed as the providers' listings tell, and as providers
///    connect and disconnect, until the client cancels.
/// 4. **Reached.** Only routes through the providers a `providers_daemons`
///    grant's `list` reaches are listed: an identity none of whose routes is
///    reached is not listed at all, and a listed identity's
///    `providers` hold only its reached routes. The reach is read again
///    at every read, so a provider's record changed — a word of
///    `Kind::Providers` — re-reads the listing as the grants now reach.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    // Rule 1.
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::providers_daemons::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    // Rules 2, 3 and 4: the watcher left as its guard drops, however
    // the scope ends.
    let _watching = daemon.live.listings.enter(daemon).await;
    let source = Merged { daemon, standing: &standing };
    stream::listing(scope, daemon, &[Kind::Listings, Kind::Providers], frame.count, &source, Frame::from).await
}

/// The merge as it is now, narrowed to the routes the grants reach.
struct Merged<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
}

impl Source for Merged<'_> {
    type Key = Identity;
    type Item = Listed;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(Identity, Listed)>, store::Error>> + Send {
        async move {
            // Rule 4: the reach as the records stand now.
            let reached = judge::providers_daemons::reached(self.standing, Over::List, &self.daemon.providers.all());
            // Only the reached providers' listings are merged: their
            // routes, and their descriptions, and no other's.
            Ok(self.daemon.live.listings.read(self.daemon, &reached).await)
        }
    }
}
