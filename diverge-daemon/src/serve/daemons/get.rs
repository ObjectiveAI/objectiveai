//! Getting one remote daemon's record.

use diverge_sdk::daemon::endpoints::daemons::get::client::request;
use diverge_sdk::daemon::endpoints::daemons::get::server::response::Frame;
use diverge_sdk::daemon::grant::daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
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

/// `Forbidden` with no `get` grant at all; `NotFound`; `Forbidden` for a
/// record the grants do not reach; else the record as a list reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::daemons::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = store::daemons::by_name(&mut conn, &frame.name, false).await? else {
        return Ok(Frame::NotFound);
    };
    let reported = record.report(daemon.live.is_daemon_connected(&record.name).await);
    if !judge::daemons::over(&standing, Over::Get, &reported) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(reported))
}
