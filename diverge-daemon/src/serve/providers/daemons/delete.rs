//! Forgetting a daemon record.

use diverge_sdk::daemon::endpoints::providers::daemons::delete::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::delete::server::response::Frame;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, in_use, providers_daemons};

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
/// for a record the grants do not reach; `InUse` while a connected
/// tool names it; else the record forgotten, the connection this
/// daemon holds to it let go, and its name free for an add.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_daemons::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = providers_daemons::by_name(&mut tx, &frame.name, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_daemon_connected(&record.name).await;
    if !judge::providers_daemons::over(&standing, Over::Delete, &record, connected) {
        return Ok(Frame::Forbidden);
    }
    if in_use::daemon(&mut tx, &record.name).await? {
        return Ok(Frame::InUse);
    }
    providers_daemons::delete(&mut tx, record.id).await?;
    tx.commit().await?;
    daemon.live.remove_daemon(&record.name).await;
    daemon.live.changed(Kind::ProvidersDaemons);
    Ok(Frame::Deleted)
}
