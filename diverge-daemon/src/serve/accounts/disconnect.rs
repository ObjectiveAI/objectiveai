//! Closing every connection a client has as an account.

use diverge_sdk::daemon::endpoints::accounts::disconnect::client::request;
use diverge_sdk::daemon::endpoints::accounts::disconnect::server::response::Frame;
use diverge_sdk::daemon::grant::accounts::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Connected, Daemon};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, accounts};

/// Answer the disconnect and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. **Judged.** `Forbidden` with no `disconnect` grant at all.
/// 2. **An account** of the identity named: `Forbidden` where the grants
///    do not reach it; else every connection a client has as it told to
///    close.
/// 3. **An administrator** the configuration names, with no account: the
///    same, judged as an administrator is.
/// 4. **Neither** is `NotFound`.
/// 5. **The answer** is how many connections were told to close, `0`
///    where none was open; it is sent without waiting for them to end, so
///    an account may disconnect itself.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    // Rule 1.
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::accounts::holds(&standing, Over::Disconnect) {
        return Ok(Frame::Forbidden);
    }
    // Rule 2.
    if let Some(account) = accounts::by_identity(&mut conn, &frame.account, false).await? {
        drop(conn);
        let connected = daemon.live.is_connected(account.id).await;
        if !judge::accounts::over(&standing, Over::Disconnect, &account, connected) {
            return Ok(Frame::Forbidden);
        }
        let closed = daemon.live.disconnect(&Connected::Account(account.id)).await;
        return Ok(Frame::Disconnected(closed as u64));
    }
    drop(conn);
    // Rule 3.
    if daemon.authenticator.is_admin(&frame.account) {
        let connected = daemon.live.admins_connected().await.contains(&frame.account);
        if !judge::accounts::over_admin(&standing, Over::Disconnect, &frame.account, connected) {
            return Ok(Frame::Forbidden);
        }
        let closed = daemon.live.disconnect(&Connected::Admin(frame.account.clone())).await;
        return Ok(Frame::Disconnected(closed as u64));
    }
    // Rule 4.
    Ok(Frame::NotFound)
}
