//! Listing outgoing providers.

use diverge_sdk::daemon::endpoints::providers::outgoing::list::client::request;
use diverge_sdk::daemon::endpoints::providers::outgoing::list::server::response::Frame;
use diverge_sdk::daemon::grant::providers_outgoing::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store::{self, providers_outgoing};

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every provider
/// any `list` grant reaches that the request's filter lets through,
/// oldest added first, at most `count` of them, one frame each — and
/// nothing at all, which is an answer, when none does. Which are
/// connected is one snapshot for the whole list.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::providers_outgoing::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let all = providers_outgoing::all(&mut conn).await?;
    drop(conn);
    let connected = daemon.live.connected_providers().await;
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let sent = all
        .iter()
        .map(|provider| (provider, connected.contains(&provider.identity())))
        .filter(|(provider, connected)| judge::providers_outgoing::over(&standing, Over::List, provider, *connected))
        .filter(|(provider, connected)| filter::providers_outgoing::test(&frame.filter, provider, *connected))
        .take(cap);
    for (provider, connected) in sent {
        reply::reply(scope, &Frame::Outgoing(provider.report(connected))).await;
    }
    Ok(())
}
