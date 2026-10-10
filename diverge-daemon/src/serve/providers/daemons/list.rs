//! Listing daemon records, and keeping the list.

use std::future::Future;

use diverge_sdk::daemon::endpoints::providers::daemons::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::providers::daemons::list::server::response::{self, Frame};
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, providers_daemons};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every daemon
/// record any `list` grant reaches that the request's filter lets
/// through, oldest added first, the first `count` of them, one frame
/// each, then the word that the listing is whole — and from then on
/// each record added, changed or removed as the records and the
/// connections change, until the client cancels. Which are connected is
/// read afresh at every change.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::providers_daemons::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::ProvidersDaemons], frame.count, &source, |change| match change {
        Change::Added(record) => Frame::Added(record),
        Change::Changed(record) => Frame::Changed(record),
        Change::Removed(record) => Frame::Removed(record),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The daemon records as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = String;
    type Item = response::Daemon;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(String, response::Daemon)>, store::Error>> + Send {
        async move {
            let all = {
                let mut conn = self.daemon.store.acquire().await?;
                providers_daemons::all(&mut conn).await?
            };
            let connected = self.daemon.live.connected_daemons().await;
            Ok(all
                .iter()
                .map(|record| (record, connected.contains(&record.name)))
                .filter(|(record, connected)| judge::providers_daemons::over(self.standing, Over::List, record, *connected))
                .filter(|(record, connected)| filter::providers_daemons::test(self.filter, record, *connected))
                .map(|(record, connected)| (record.name.clone(), record.report(connected)))
                .collect())
        }
    }
}
