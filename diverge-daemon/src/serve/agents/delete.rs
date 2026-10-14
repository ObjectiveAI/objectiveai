//! Deleting an agent.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::delete::client::request;
use diverge_sdk::daemon::endpoints::agents::delete::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Failure, active};
use crate::containers;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::logs;
use crate::serve::reply;
use crate::database;
use crate::store::{agents, dependency_tools, tools};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden`
/// for an agent the grants do not reach; `Active` while a loop runs
/// in it; else the agent gone — its container stopped if it was up,
/// its attachments with it, the dependency tools it owns gone with
/// their database scopes and their every table, each service among
/// them retired, every other dependency tool and every tool on record
/// it was attached to told,
/// its name free, every watch on its log ended and the log removed.
/// Its own database scope stays: nothing drops an agent's own.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, Failure> {
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
    let attached = dependency_tools::of_agent(&mut tx, agent.id).await?;
    let detached = tools::attachments::of_agent(&mut tx, agent.id).await?;
    for tool in dependency_tools::of_owner_agent(&mut tx, agent.id).await? {
        if let Some(container) = database::container_of_dependency_tool(&tool, Some(&agent)) {
            database::provision::drop(&mut tx, &database::role_of(&container)).await?;
        }
    }
    let removed = dependency_tools::delete_of_owner_agent(&mut tx, agent.id).await?;
    agents::delete(&mut tx, agent.id).await?;
    tx.commit().await?;
    daemon.live.forget_scope(containers::Key::Agent(agent.id)).await;
    containers::stop_agent(daemon, agent.id).await;
    let left = attached
        .iter()
        .filter(|tool| !removed.iter().any(|gone| gone.id == tool.id))
        .map(|tool| tool.id)
        .collect();
    containers::services::agent_deleted(daemon, removed, left, detached).await;
    daemon.live.end_log(agent.id).await;
    logs::remove(&daemon.logs, agent.id).await?;
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::StandaloneTools);
    daemon.live.changed(Kind::DependencyTools);
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Deleted)
}
