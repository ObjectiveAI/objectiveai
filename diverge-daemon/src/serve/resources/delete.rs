//! Deleting a resource.

use diverge_sdk::daemon::endpoints::resources::delete::client::request;
use diverge_sdk::daemon::endpoints::resources::delete::server::response::Frame;
use diverge_sdk::daemon::grant::resources::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::in_use;
use crate::content;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, resources};

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
/// for a resource the grants do not reach; `InUse` while some container
/// mounts it; else the resource deleted — its row kept, so the same
/// bytes held anew are this one again — and its bytes removed.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::resources::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = resources::by_id(&mut tx, &frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let held = store::in_use::resources(&mut tx).await?;
    let used = in_use(&held, &record);
    if !judge::resources::over(&standing, Over::Delete, &record, used) {
        return Ok(Frame::Forbidden);
    }
    if used {
        return Ok(Frame::InUse);
    }
    resources::delete(&mut tx, &record.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Resources);
    // The record is gone first, so a reader that finds no record never
    // finds the bytes either; bytes that will not go are left, and the
    // next hold of the same content finds them in place.
    let _ = content::remove(&daemon.resources, &record.id).await;
    Ok(Frame::Deleted)
}
