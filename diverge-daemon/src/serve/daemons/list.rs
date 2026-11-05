//! Listing the remote daemons' records, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::daemons::Daemon as Record;
use diverge_sdk::daemon::endpoints::daemons::list::client::request;
use diverge_sdk::daemon::endpoints::daemons::list::server::response::Frame;
use diverge_sdk::daemon::grant::daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::serve::stream::{self, Source};
use crate::store;

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant; else every record the request's
/// filter lets through and a grant of `list` reaches, by name, the first
/// `count` of them, one frame each, then the word that the listing is
/// whole — and from then on each record added, changed or removed as the
/// records and their connections change, until the client cancels.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::daemons::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed { daemon, standing: &standing, filter: &frame.filter };
    stream::listing(scope, daemon, &[Kind::Daemons], frame.count, &source, Frame::from).await
}

/// The records as the filter and the grants let them through now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a request::Filter,
}

impl Source for Listed<'_> {
    type Key = String;
    type Item = Record;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(String, Record)>, store::Error>> + Send {
        async move {
            let records = {
                let mut conn = self.daemon.store.acquire().await?;
                store::daemons::all(&mut conn).await?
            };
            let connected = self.daemon.live.connected_daemons().await;
            Ok(records
                .iter()
                .map(|record| record.report(connected.contains(&record.name)))
                .filter(|reported| filter::daemons::test(self.filter, reported) && judge::daemons::over(self.standing, Over::List, reported))
                .map(|reported| (reported.name.clone(), reported))
                .collect())
        }
    }
}
