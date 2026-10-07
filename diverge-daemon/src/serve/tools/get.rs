//! Getting one tool.

use diverge_sdk::daemon::endpoints::tools::get::client::request;
use diverge_sdk::daemon::endpoints::tools::get::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of, report};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tools};

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
    if !judge::tools::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut conn, &frame.tool, false).await? else {
        return Ok(Frame::NotFound);
    };
    let attached = agents_of(&mut conn, tool.id).await?;
    if !judge::tools::over(&standing, Over::Get, &tool, active(daemon, tool.id).await, &attached) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(report(&mut conn, daemon, &tool).await?))
}
