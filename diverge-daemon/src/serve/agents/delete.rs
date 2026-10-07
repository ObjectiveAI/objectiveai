//! Deleting an agent.

use diverge_sdk::daemon::endpoints::agents::delete::client::request;
use diverge_sdk::daemon::endpoints::agents::delete::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Failure, active};
use crate::containers;
use crate::daemon::Daemon;
use crate::database;
use crate::judge::{self, Standing, Who};
use crate::logs;
use crate::serve::reply;
use crate::store::agents;

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden`
/// for an agent the grants do not reach; `Active` while a loop runs
/// in it; else the agent gone — its database scope dropped with its
/// every table, its container stopped if it was up, its attachments
/// with it, its name free, every watch on its log ended and the log
/// removed.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, Failure> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NotFound);
    };
    let active = active(daemon, agent.id).await;
    if !judge::agents::over(&standing, Over::Delete, &agent, active) {
        return Ok(Frame::Forbidden);
    }
    if active {
        return Ok(Frame::Active);
    }
    agents::delete(&mut tx, agent.id).await?;
    database::provision::drop(&mut tx, &database::role_of(&database::container_of_agent(&agent))).await?;
    tx.commit().await?;
    daemon.live.forget_scope(containers::Key::Agent(agent.id)).await;
    containers::stop_agent(daemon, agent.id).await;
    daemon.live.end_log(agent.id).await;
    logs::remove(&daemon.logs, agent.id).await?;
    Ok(Frame::Deleted)
}
