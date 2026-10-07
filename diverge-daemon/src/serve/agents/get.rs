//! Getting one agent.

use diverge_sdk::daemon::endpoints::agents::get::client::request;
use diverge_sdk::daemon::endpoints::agents::get::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Failure, active, tools_of};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::logs;
use crate::serve::reply;
use crate::store::agents;

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
/// for an agent the grants do not reach; else the agent as a list
/// reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, Failure> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        return Ok(Frame::NotFound);
    };
    let active = active(daemon, agent.id).await;
    if !judge::agents::over(&standing, Over::Get, &agent, active) {
        return Ok(Frame::Forbidden);
    }
    let tools = tools_of(&mut conn, agent.id).await?;
    drop(conn);
    let logs_index = logs::count(&daemon.logs, agent.id).await?;
    Ok(Frame::Found(agent.report(active, tools, logs_index)))
}
