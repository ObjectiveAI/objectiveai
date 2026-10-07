//! Listing incoming credentials.

use diverge_sdk::daemon::endpoints::providers::incoming::list::client::request;
use diverge_sdk::daemon::endpoints::providers::incoming::list::server::response::Frame;
use diverge_sdk::daemon::grant::providers_incoming::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store::{self, providers_incoming};

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every credential
/// any `list` grant reaches that the request's filter lets through,
/// oldest added first, at most `count` of them, one frame each — and
/// nothing at all, which is an answer, when none does.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::providers_incoming::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let all = providers_incoming::all(&mut conn).await?;
    drop(conn);
    let connected = daemon.live.connected_providers().await;
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let sent = all
        .iter()
        .map(|credential| (credential, connected.contains(&credential.provider())))
        .filter(|(credential, connected)| judge::providers_incoming::over(&standing, Over::List, credential, *connected))
        .filter(|(credential, connected)| filter::providers_incoming::test(&frame.filter, credential, *connected))
        .take(cap);
    for (credential, connected) in sent {
        reply::reply(scope, &Frame::Incoming(credential.report(connected))).await;
    }
    Ok(())
}
