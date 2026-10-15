//! Deleting a role.

use diverge_sdk::daemon::endpoints::roles::delete::client::request;
use diverge_sdk::daemon::endpoints::roles::delete::server::response::Frame;
use diverge_sdk::daemon::grant::roles::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, roles};

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
/// for a role the grants do not reach; `InUse` while an account holds
/// it; else the role gone.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::roles::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(role) = roles::by_name(&mut tx, &frame.name, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::roles::over(&standing, Over::Delete, &role) {
        return Ok(Frame::Forbidden);
    }
    match roles::delete(&mut tx, role.id).await? {
        roles::Deleted::Deleted => {}
        roles::Deleted::InUse => return Ok(Frame::InUse),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Roles);
    Ok(Frame::Deleted)
}
