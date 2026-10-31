//! Detaching a tool from one agent.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::standalone_tools::detach_agent_instance::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::detach_agent_instance::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over as AgentsOver;
use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::containers::{self, ToolKey, services};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::agents;
use crate::serve::reply;
use crate::store::tools::attachments;
use crate::store::{self, agents as agent_records, tools};

/// Answer the detach and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `detach` grant over tools or no `edit` grant
/// over agents at all; `NoTool`; `NoAgent`; `Forbidden` for a tool or
/// an agent the grants do not reach; else the tool detached, one not
/// attached nothing to take back — on an active agent as well as an
/// idle one: unless the agent has the tool through its template
/// still, its proxy serves the tool no more from now on and tells the
/// agent its tool list changed, a call in flight answered still. A
/// service is told its hold changed.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::standalone_tools::holds(&standing, Over::Detach) || !judge::agents::holds(&standing, AgentsOver::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NoTool);
    };
    let Some(agent) = agent_records::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NoAgent);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Detach, &tool).await?
        || !judge::agents::over(&standing, AgentsOver::Edit, &agent, agents::state(daemon, agent.id).await)
    {
        return Ok(Frame::Forbidden);
    }
    attachments::detach(&mut tx, tool.id, agent.id).await?;
    let keeps = tools::attached_agents(&mut tx, tool.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::StandaloneTools);
    services::held(daemon, ToolKey::Record(tool.id)).await;
    containers::detached(daemon, tool.id, &[agent.id], &keeps).await;
    Ok(Frame::Detached)
}
