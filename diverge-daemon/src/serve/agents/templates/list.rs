//! Listing agent templates.

use diverge_sdk::daemon::grant::agents_templates::Over;
use diverge_sdk::daemon::endpoints::agents::templates::list::client::request;
use diverge_sdk::daemon::endpoints::agents::templates::list::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store::{self, agents_templates};
use super::in_use;

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every template
/// any `list` grant reaches that the request's filter lets through,
/// oldest first, at most `count` of them, one frame each — and nothing
/// at all, which is an answer, when none does.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents_templates::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let all = agents_templates::all(&mut conn).await?;
    let held = store::in_use::agents_templates(&mut conn).await?;
    drop(conn);
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let sent = all
        .iter()
        .map(|record| (record, in_use(&held, record)))
        .filter(|(record, in_use)| judge::agents_templates::over(&standing, Over::List, record, *in_use))
        .filter(|(record, in_use)| filter::agents_templates::test(&frame.filter, record, *in_use))
        .take(cap);
    for (record, _) in sent {
        reply::reply(scope, &Frame::Listed(record.report())).await;
    }
    Ok(())
}
