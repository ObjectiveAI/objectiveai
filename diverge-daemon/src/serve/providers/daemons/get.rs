//! Getting one daemon record.

use diverge_sdk::daemon::endpoints::providers::daemons::get::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::get::server::response::Frame;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_daemons};

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `get` grant at all; `NotFound`; `Forbidden` for
/// a record the grants do not reach; else the record as a list reports
/// it, never its credential.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_daemons::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = providers_daemons::by_name(&mut conn, &frame.name, false).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_daemon_connected(&record.name).await;
    if !judge::providers_daemons::over(&standing, Over::Get, &record, connected) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(record.report(connected)))
}
