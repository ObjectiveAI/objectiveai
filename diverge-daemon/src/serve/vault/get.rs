//! Getting one vault key, with its value.

use diverge_sdk::daemon::endpoints::vault::get::client::request;
use diverge_sdk::daemon::endpoints::vault::get::server::response::Frame;
use diverge_sdk::daemon::grant::vault::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, vault};

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
/// a key the grants do not reach; else the key with its value.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::vault::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(entry) = vault::by_key(&mut conn, &frame.key, false).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::vault::over(&standing, Over::Get, &entry) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(entry.report_with_value()))
}
