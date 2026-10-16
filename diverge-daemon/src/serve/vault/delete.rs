//! Deleting a vault key.

use diverge_sdk::daemon::endpoints::vault::delete::client::request;
use diverge_sdk::daemon::endpoints::vault::delete::server::response::Frame;
use diverge_sdk::daemon::grant::vault::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, vault};

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
/// for a key the grants do not reach; `InUse` while an agent or a
/// tool binds it; else the key gone.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::vault::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(entry) = vault::by_key(&mut tx, &frame.key, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::vault::over(&standing, Over::Delete, &entry) {
        return Ok(Frame::Forbidden);
    }
    match vault::delete(&mut tx, &frame.key).await? {
        vault::Deleted::Deleted => {}
        vault::Deleted::InUse => return Ok(Frame::InUse),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Vault);
    Ok(Frame::Deleted)
}
