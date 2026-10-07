//! Getting one resource.

use diverge_sdk::daemon::endpoints::resources::get::client::request;
use diverge_sdk::daemon::endpoints::resources::get::server::response::Frame;
use diverge_sdk::daemon::grant::resources::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::in_use;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, resources};

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
/// a resource the grants do not reach; else the resource as a list
/// reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::resources::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = resources::by_id(&mut conn, &frame.id, false).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::resources::over(&standing, Over::Get, &record, in_use(&record)) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(record.report()))
}
