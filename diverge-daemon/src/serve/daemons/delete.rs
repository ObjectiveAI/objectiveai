//! Deleting a remote daemon's record.

use diverge_sdk::daemon::endpoints::daemons::delete::client::request;
use diverge_sdk::daemon::endpoints::daemons::delete::server::response::Frame;
use diverge_sdk::daemon::grant::daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the request and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden` for
/// a record the grants do not reach; `InUse` while a connected tool names
/// it, those deleted first; else the record gone, its connection — if
/// one is held — dropped, and the lists told.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::daemons::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = store::daemons::by_id(&mut tx, frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let reported = record.report(daemon.live.is_daemon_connected(record.id).await);
    if !judge::daemons::over(&standing, Over::Delete, &reported) {
        return Ok(Frame::Forbidden);
    }
    if store::in_use::daemon(&mut tx, record.id).await? {
        return Ok(Frame::InUse);
    }
    store::daemons::delete(&mut tx, record.id).await?;
    tx.commit().await?;
    if let Some(peer) = daemon.live.daemon_peer(record.id).await {
        daemon.live.remove_peer(&peer).await;
    }
    daemon.live.changed(Kind::Daemons);
    Ok(Frame::Deleted)
}
