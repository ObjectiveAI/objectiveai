//! Getting one role.

use diverge_sdk::daemon::endpoints::roles::get::client::request;
use diverge_sdk::daemon::endpoints::roles::get::server::response::Frame;
use diverge_sdk::daemon::grant::roles::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, roles};

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
/// a role the grants do not reach; else the role as a list reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::roles::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(role) = roles::by_name(&mut conn, &frame.name, false).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::roles::over(&standing, Over::Get, &role) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(role.report()))
}
