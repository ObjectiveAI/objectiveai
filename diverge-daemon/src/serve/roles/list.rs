//! Listing roles.

use diverge_sdk::daemon::endpoints::roles::list::client::request;
use diverge_sdk::daemon::endpoints::roles::list::server::response::Frame;
use diverge_sdk::daemon::grant::roles::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store::{self, roles};

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every role any
/// `list` grant reaches that the request's filter lets through,
/// oldest created first, at most `count` of them, one frame each —
/// and nothing at all, which is an answer, when none does.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::roles::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let all = roles::all(&mut conn).await?;
    drop(conn);
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let sent = all
        .iter()
        .filter(|role| judge::roles::over(&standing, Over::List, role))
        .filter(|role| filter::roles::test(&frame.filter, role))
        .take(cap);
    for role in sent {
        reply::reply(scope, &Frame::Role(role.report())).await;
    }
    Ok(())
}
