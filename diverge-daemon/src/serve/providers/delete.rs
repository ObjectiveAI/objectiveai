//! Deleting a provider's record.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::providers::delete::client::request;
use diverge_sdk::daemon::endpoints::providers::delete::server::response::Frame;
use diverge_sdk::daemon::grant::providers::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::providers;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the request and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden`
/// for a record the grants do not reach; `Enabled` while the
/// record is enabled, since a delete stops no work; `InUse` while an
/// agent or a tool is pinned to the provider or mounts its volumes;
/// else the record and its volumes' tags gone, the book read again, and
/// the lists told. The row is held for the transaction, so no pin lands
/// between the check and the delete.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = store::providers::by_identity(&mut tx, &frame.identity, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::providers::over(&standing, Over::Delete, &record) {
        return Ok(Frame::Forbidden);
    }
    if record.enabled {
        return Ok(Frame::Enabled);
    }
    if store::providers::in_use(&mut tx, &record.identity).await? {
        return Ok(Frame::InUse);
    }
    store::providers::delete(&mut tx, &record.identity).await?;
    tx.commit().await?;
    daemon.providers.reload(&daemon.store).await?;
    providers::reconcile(daemon, &record.identity, None).await;
    daemon.live.changed(Kind::Providers);
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Deleted)
}
