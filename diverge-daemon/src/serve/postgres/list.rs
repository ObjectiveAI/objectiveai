//! The container connections open through the database, and kept.

use std::future::Future;

use diverge_sdk::daemon::endpoints::postgres::Connection;
use diverge_sdk::daemon::endpoints::postgres::list::client::request;
use diverge_sdk::daemon::endpoints::postgres::list::server::response::Frame;
use diverge_sdk::daemon::grant::postgres::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::database;
use crate::judge::{self, Standing, Who};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store;

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` without the `list` grant; else every connection a
/// container holds open now, oldest opened first, one frame each, the
/// container named as the database names it, then the word that the
/// list is whole — at once, when none is open — and from then on each
/// connection added as it opens and removed as it closes, until the
/// client cancels. One whose container's record is gone is not
/// listed; one whose container is renamed is changed.
async fn serve(scope: &ScopeHandle, _: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::postgres::holds(&standing, Action::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Open { daemon };
    stream::listing(scope, daemon, &[Kind::Postgres, Kind::Agents, Kind::Tools], None, &source, |change| match change {
        Change::Added(connection) => Frame::Added(connection),
        Change::Changed(connection) => Frame::Changed(connection),
        Change::Removed(connection) => Frame::Removed(connection),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The connections open now, each by the daemon's own number for it.
struct Open<'a> {
    daemon: &'a Daemon,
}

impl Source for Open<'_> {
    type Key = u64;
    type Item = Connection;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(u64, Connection)>, store::Error>> + Send {
        async move {
            let open = self.daemon.live.connections().await;
            let mut conn = self.daemon.store.acquire().await?;
            let mut listed = Vec::with_capacity(open.len());
            for (id, key, opened) in open {
                if let Some(container) = database::container_of(&mut conn, key).await? {
                    listed.push((id, Connection { container, opened }));
                }
            }
            Ok(listed)
        }
    }
}
