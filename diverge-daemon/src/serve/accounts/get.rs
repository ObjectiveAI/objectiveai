//! Getting one account.

use diverge_sdk::daemon::endpoints::accounts::get::client::request;
use diverge_sdk::daemon::endpoints::accounts::get::server::response::Frame;
use diverge_sdk::daemon::grant::accounts::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, accounts};

use super::admins;

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `get` grant at all, before anything is looked
/// at; an administrator the configuration names with no account of its
/// own as its entry ([`admins`](super::admins)); `NotFound` for a
/// reference naming no account; `Forbidden` for
/// one the grants do not reach; else the account as a list reports
/// it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::accounts::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(account) = accounts::by_identity(&mut conn, &frame.account, false).await? else {
        // Admins rule 1: an administrator with no account of its own is
        // found as its entry.
        if daemon.authenticator.is_admin(&frame.account) {
            let connected = daemon.live.admins_connected().await.contains(&frame.account);
            if !judge::accounts::over_admin(&standing, Over::Get, &frame.account, connected) {
                return Ok(Frame::Forbidden);
            }
            return Ok(Frame::Found(admins::entry(&frame.account, connected, daemon.live.admin_last_connected(&frame.account).await)));
        }
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_connected(account.id).await;
    if !judge::accounts::over(&standing, Over::Get, &account, connected) {
        return Ok(Frame::Forbidden);
    }
    // Admins rule 2.
    let mut reported = account.report(connected);
    reported.admin = daemon.authenticator.is_admin(&account.identity);
    Ok(Frame::Found(reported))
}
