//! Deleting an account.

use diverge_sdk::daemon::endpoints::accounts::delete::client::request;
use diverge_sdk::daemon::endpoints::accounts::delete::server::response::Frame;
use diverge_sdk::daemon::grant::accounts::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, accounts};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden`
/// for an account the grants do not reach; `InUse` while a client is
/// connected as it or a container runs under it; else the account
/// gone, its holdings of roles with it, and what it created keeping
/// its creator.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::accounts::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(account) = accounts::by_reference(&mut tx, &frame.account, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_connected(account.id).await;
    if !judge::accounts::over(&standing, Over::Delete, &account, connected) {
        return Ok(Frame::Forbidden);
    }
    if daemon.live.holds(account.id).await {
        return Ok(Frame::InUse);
    }
    accounts::delete(&mut tx, account.id).await?;
    tx.commit().await?;
    Ok(Frame::Deleted)
}
