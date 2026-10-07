//! Detaching a tool from an agent.

use diverge_sdk::daemon::endpoints::tools::detach::client::request;
use diverge_sdk::daemon::endpoints::tools::detach::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over as AgentsOver;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::containers;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::agents;
use crate::serve::reply;
use crate::store::tools::attachments;
use crate::store::{self, agents as agent_records, tools};

/// Answer the detach and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `detach` grant over tools or no `edit` grant
/// over agents at all; `NoTool`; `NoAgent`; `Forbidden` for a tool or
/// an agent the grants do not reach; `Active` while a loop runs in
/// the agent, which keeps its tools until the loop ends; else the
/// tool detached, one not attached nothing to take back.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Detach) || !judge::agents::holds(&standing, AgentsOver::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NoTool);
    };
    let Some(agent) = agent_records::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NoAgent);
    };
    let attached = agents_of(&mut tx, tool.id).await?;
    let agent_active = agents::active(daemon, agent.id).await;
    if !judge::tools::over(&standing, Over::Detach, &tool, active(daemon, tool.id).await, &attached)
        || !judge::agents::over(&standing, AgentsOver::Edit, &agent, agent_active)
    {
        return Ok(Frame::Forbidden);
    }
    if agent_active {
        return Ok(Frame::Active);
    }
    attachments::detach(&mut tx, tool.id, agent.id).await?;
    tx.commit().await?;
    if let Some(run) = daemon.live.agent_run(agent.id).await
        && run.served.lock().await.remove(tool.id).is_some()
    {
        containers::release(daemon, tool.id, containers::Key::Agent(agent.id)).await;
    }
    Ok(Frame::Detached)
}
