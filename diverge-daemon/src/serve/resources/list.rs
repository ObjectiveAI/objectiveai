//! Listing resources, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::resources::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::resources::list::server::response::{Frame, Listed};
use diverge_sdk::daemon::grant::resources::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::in_use;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, resources};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every resource
/// any `list` grant reaches that the request's filter lets through,
/// oldest first, the first `count` of them, one frame each, then the
/// word that the list is whole — and from then on each resource added,
/// changed or removed as resources are held, described, tagged and
/// deleted, and as the containers that mount them come and go, until
/// the client cancels.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::resources::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Held {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Resources, Kind::Agents, Kind::Tools], frame.count, &source, |change| match change {
        Change::Added(resource) => Frame::Added(resource),
        Change::Changed(resource) => Frame::Changed(resource),
        Change::Removed(resource) => Frame::Removed(resource),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The resources as the caller may list them now.
struct Held<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Held<'_> {
    type Key = String;
    type Item = Listed;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(String, Listed)>, store::Error>> + Send {
        async move {
            let (all, held) = {
                let mut conn = self.daemon.store.acquire().await?;
                (resources::all(&mut conn).await?, store::in_use::resources(&mut conn).await?)
            };
            Ok(all
                .iter()
                .map(|record| (record, in_use(&held, record)))
                .filter(|(record, in_use)| judge::resources::over(self.standing, Over::List, record, *in_use))
                .filter(|(record, in_use)| filter::resources::test(self.filter, record, *in_use))
                .map(|(record, _)| (record.id.clone(), record.report()))
                .collect())
        }
    }
}
