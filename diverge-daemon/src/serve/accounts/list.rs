//! Listing accounts.

use diverge_sdk::daemon::endpoints::accounts::list::client::request;
use diverge_sdk::daemon::endpoints::accounts::list::server::response::Frame;
use diverge_sdk::daemon::grant::accounts::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store::{self, accounts};

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every account
/// any `list` grant reaches that the request's filter lets through,
/// oldest created first, at most `count` of them, one frame each —
/// and nothing at all, which is an answer, when none does. Whether a
/// client is connected is one snapshot for the whole list, taken
/// before it, and a failure before anything was sent is one `Error`.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::accounts::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let all = accounts::all(&mut conn).await?;
    drop(conn);
    let connected = daemon.live.connected().await;
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let sent = all
        .iter()
        .map(|account| (account, connected.contains(&account.id)))
        .filter(|(account, connected)| judge::accounts::over(&standing, Over::List, account, *connected))
        .filter(|(account, connected)| filter::accounts::test(&frame.filter, account, *connected))
        .take(cap);
    for (account, connected) in sent {
        reply::reply(scope, &Frame::Account(account.report(connected))).await;
    }
    Ok(())
}
