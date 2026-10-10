//! Listing volumes, provider by provider, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::volumes::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::volumes::list::server::response::{self, Frame};
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::daemon::reference;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use sqlx::PgConnection;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, providers_incoming, providers_outgoing, volumes as volume_tags};
use crate::volumes::{self, Listed, Whole};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `list` grant at all; else the providers the
/// grants reach — every one on record when some grant is `any` or
/// names none, else the ones named — in record order, each connected
/// and listed, each volume of its mirror by name, judged and
/// filtered, the first `count` sent, then the word that the list is
/// whole — and from then on each volume added, changed or removed as
/// the providers' listings, the connections, the mounting records and
/// the tags change, until the client cancels. The tags are one load
/// for the whole list, at every reading. A provider on record but not
/// connected, or not yet listed, contributes nothing until it is;
/// one whose listing failed contributes nothing until it lists again.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::volumes::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Mirrored {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(
        scope,
        daemon,
        &[Kind::Volumes, Kind::ProvidersOutgoing, Kind::ProvidersIncoming, Kind::Agents, Kind::Tools],
        frame.count,
        &source,
        |change| match change {
            Change::Added(volume) => Frame::Added(volume),
            Change::Changed(volume) => Frame::Changed(volume),
            Change::Removed(volume) => Frame::Removed(volume),
            Change::Listed => Frame::Listed,
        },
    )
    .await
}

/// The volumes as the caller may list them now, out of the mirrors.
struct Mirrored<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Mirrored<'_> {
    type Key = reference::Volume;
    type Item = response::Volume;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(reference::Volume, response::Volume)>, store::Error>> + Send {
        async move {
            let named = judge::volumes::providers(self.standing, Over::List);
            let mut conn = self.daemon.store.acquire().await?;
            let providers: Vec<Identity> = on_record(&mut conn)
                .await?
                .into_iter()
                .filter(|identity| named.as_ref().is_none_or(|named| named.contains(identity)))
                .collect();
            let mut tagged = volume_tags::all(&mut conn).await?;
            let mut listed = Vec::new();
            for identity in providers {
                let Some(mirror) = self.daemon.live.mirror(&identity).await else {
                    continue;
                };
                if mirror.whole() != Whole::Listed {
                    continue;
                }
                for volume in mirror.volumes().await {
                    let reference = reference::Volume {
                        provider: identity.clone(),
                        name: volume.name.clone(),
                    };
                    let (agents, tools) = volumes::mounters(&mut conn, &reference).await?;
                    let item = Listed {
                        provider: identity.clone(),
                        volume,
                        agents,
                        tools,
                        tags: tagged.remove(&reference).unwrap_or_default(),
                    };
                    if !judge::volumes::over(self.standing, Over::List, &item) || !filter::volumes::test(self.filter, &item) {
                        continue;
                    }
                    listed.push((reference, item.report()));
                }
            }
            Ok(listed)
        }
    }
}

/// Every provider on record, outgoing then incoming, each in its
/// record order.
async fn on_record(conn: &mut PgConnection) -> Result<Vec<Identity>, store::Error> {
    let mut identities: Vec<Identity> = providers_outgoing::all(conn).await?.iter().map(|outgoing| outgoing.identity()).collect();
    identities.extend(providers_incoming::all(conn).await?.iter().map(|incoming| incoming.provider()));
    Ok(identities)
}
