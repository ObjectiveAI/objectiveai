//! Listing the images in the daemon's Stores, provider by provider,
//! and keeping the list.

use std::future::Future;

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::daemon::endpoints::images::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::images::list::server::response::{self, Frame};
use diverge_sdk::daemon::grant::images::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::serve::stream::{self, Source};
use crate::store;
use crate::volumes::Whole;

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. **Judged.** One `Forbidden` with no `list` grant over images
///    reaching any provider at all.
/// 2. **The list.** Every provider on record a `list` grant reaches,
///    in record order, each connected and its Store's listing whole,
///    each image of its mirror by digest, filtered, the first `count`
///    sent, then the word that the list is whole — and from then on
///    each image added, changed or removed as the providers' listings,
///    the connections and the providers' records change, until the
///    client cancels.
/// 3. **Reached again at every read.** The reach is read from the
///    records as they stand, so a provider's record changed — a word
///    of `Kind::Providers` — re-reads the list as the grants now
///    reach.
/// 4. **Nothing from what is not there.** A provider on record but not
///    connected, with no Store, or not yet listed, contributes nothing
///    until it is; one whose listing failed contributes nothing until
///    it connects and lists again.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    // Rule 1.
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::images::holds(&standing, Action::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Mirrored {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Images, Kind::Providers], frame.count, &source, Frame::from).await
}

/// The images as the caller may list them now, out of the mirrors.
struct Mirrored<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Mirrored<'_> {
    type Key = (Identity, String);
    type Item = response::Image;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<((Identity, String), response::Image)>, store::Error>> + Send {
        async move {
            let mut listed = Vec::new();
            // Rules 2 and 3: the records in their order, as they stand.
            for record in self.daemon.providers.all().iter() {
                if !judge::images::over(self.standing, Action::List, record) {
                    continue;
                }
                // Rule 4.
                let Some(mirror) = self.daemon.live.store(&record.identity).await else {
                    continue;
                };
                if mirror.whole() != Whole::Listed {
                    continue;
                }
                for image in mirror.images().await {
                    let item = response::Image {
                        provider: record.identity.clone(),
                        digest: image.digest,
                        description: image.description,
                        bytes: image.bytes,
                        created: DateTime::<Utc>::from_timestamp(i64::try_from(image.created).unwrap_or(0), 0).unwrap_or_default(),
                    };
                    if !filter::images::test(self.filter, &item) {
                        continue;
                    }
                    listed.push(((item.provider.clone(), item.digest.clone()), item));
                }
            }
            Ok(listed)
        }
    }
}
