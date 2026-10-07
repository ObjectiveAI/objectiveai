//! Listing routes.

use std::collections::HashMap;

use diverge_sdk::daemon::endpoints::tools::routes::list::client::request;
use diverge_sdk::daemon::endpoints::tools::routes::list::server::response::Frame;
use diverge_sdk::daemon::grant::routes::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store::{self, routes, tools};

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every route any
/// `list` grant reaches that the request's filter lets through, oldest
/// first, at most `count` of them, one frame each, its tool as it is
/// called now — and nothing at all, which is an answer, when none
/// does.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::routes::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let all = routes::all(&mut conn).await?;
    let snapshots: HashMap<_, _> = tools::all(&mut conn)
        .await?
        .into_iter()
        .filter_map(|tool| tool.snapshot().map(|snapshot| (tool.id, snapshot)))
        .collect();
    drop(conn);
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let sent = all
        .iter()
        .filter_map(|route| snapshots.get(&route.tool).map(|tool| (route, tool)))
        .filter(|(route, tool)| judge::routes::over(&standing, Over::List, route, tool.name.as_deref()))
        .filter(|(route, tool)| filter::routes::test(&frame.filter, route, tool.name.as_deref()))
        .take(cap);
    for (route, tool) in sent {
        reply::reply(scope, &Frame::Route(route.report(tool.clone()))).await;
    }
    Ok(())
}
