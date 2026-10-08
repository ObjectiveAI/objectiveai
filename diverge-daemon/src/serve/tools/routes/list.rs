//! Listing routes, and keeping the list.

use std::collections::HashMap;
use std::future::Future;

use diverge_sdk::daemon::endpoints::tools::routes::Path;
use diverge_sdk::daemon::endpoints::tools::routes::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::tools::routes::list::server::response::{Frame, Route};
use diverge_sdk::daemon::grant::routes::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, routes, tools};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every route any
/// `list` grant reaches that the request's filter lets through, oldest
/// first, the first `count` of them, one frame each, its tool as it is
/// called now, then the word that the list is whole — and from then
/// on each route added, changed or removed as routes are set and
/// deleted and tools are renamed and deleted, until the client
/// cancels.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::routes::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Routes, Kind::Tools], frame.count, &source, |change| match change {
        Change::Added(route) => Frame::Added(route),
        Change::Changed(route) => Frame::Changed(route),
        Change::Removed(route) => Frame::Removed(route),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The routes as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = Path;
    type Item = Route;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(Path, Route)>, store::Error>> + Send {
        async move {
            let (all, snapshots) = {
                let mut conn = self.daemon.store.acquire().await?;
                let all = routes::all(&mut conn).await?;
                let snapshots: HashMap<_, _> = tools::all(&mut conn)
                    .await?
                    .into_iter()
                    .filter_map(|tool| tool.snapshot().map(|snapshot| (tool.id, snapshot)))
                    .collect();
                (all, snapshots)
            };
            Ok(all
                .iter()
                .filter_map(|route| snapshots.get(&route.tool).map(|tool| (route, tool)))
                .filter(|(route, tool)| judge::routes::over(self.standing, Over::List, route, tool.name.as_deref()))
                .filter(|(route, tool)| filter::routes::test(self.filter, route, tool.name.as_deref()))
                .map(|(route, tool)| (route.path(), route.report(tool.clone())))
                .collect())
        }
    }
}
