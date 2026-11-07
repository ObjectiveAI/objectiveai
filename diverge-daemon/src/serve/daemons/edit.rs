//! Changing a remote daemon's record: its description.

use diverge_sdk::daemon::endpoints::daemons::edit::client::request;
use diverge_sdk::daemon::endpoints::daemons::edit::server::response::Frame;
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

/// `Forbidden` with no `edit` grant at all; `NotFound`; `Forbidden` for
/// a record the grants do not reach; else the record's description
/// changed, where one is given, and the lists told. What the record
/// reaches and as whom is its ID and never changes, so a connection held
/// for it is kept.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::daemons::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = store::daemons::by_id(&mut tx, frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let reported = record.report(daemon.live.is_daemon_connected(record.id).await);
    if !judge::daemons::over(&standing, Over::Edit, &reported) {
        return Ok(Frame::Forbidden);
    }
    if let Some(description) = frame.description {
        store::daemons::edit(&mut tx, record.id, &description).await?;
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Daemons);
    Ok(Frame::Edited)
}
