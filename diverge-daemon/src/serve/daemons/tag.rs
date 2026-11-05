//! Tagging a remote daemon's record.

use diverge_sdk::daemon::endpoints::daemons::tag::client::request;
use diverge_sdk::daemon::endpoints::daemons::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
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

/// `Forbidden` with no `tag` grant at all; `NotFound`; `Forbidden` for
/// a record the grants do not reach, or a tag the grant does not cover;
/// else the tags on the record, and the lists told.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::daemons::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = store::daemons::by_name(&mut tx, &frame.name, true).await? else {
        return Ok(Frame::NotFound);
    };
    let reported = record.report(daemon.live.is_daemon_connected(&record.name).await);
    if !judge::daemons::tagging(&standing, Tagging::Tag, &reported, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    store::daemons::set_tags(&mut tx, &record.name, &store::tags::with(&record.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Daemons);
    Ok(Frame::Tagged)
}
