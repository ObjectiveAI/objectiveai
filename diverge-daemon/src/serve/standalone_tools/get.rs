//! Getting one tool.

use diverge_sdk::daemon::endpoints::standalone_tools::get::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::get::server::response::Frame;
use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, report, resolve};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `get` grant at all; `NotFound`; `Forbidden`
/// for a tool the grants do not reach; else the tool as a list
/// reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::standalone_tools::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut conn, &frame.tool, false).await? else {
        return Ok(Frame::NotFound);
    };
    if !reaches(&mut conn, daemon, &standing, Over::Get, &tool).await? {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(report(&mut conn, daemon, &tool).await?))
}
