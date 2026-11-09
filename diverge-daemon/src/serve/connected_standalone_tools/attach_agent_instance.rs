//! Attaching a connected tool to one agent.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::connected_standalone_tools::attach_agent_instance::client::request;
use diverge_sdk::daemon::endpoints::connected_standalone_tools::attach_agent_instance::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over as AgentsOver;
use diverge_sdk::daemon::grant::connected_standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::containers;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::agents;
use crate::serve::reply;
use crate::store::connected_standalone_tools::attachments;
use crate::store::{self, agents as agent_records};

/// Answer the attach and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `attach` grant over connected tools or no `edit`
/// grant over agents at all; `NoTool`; `NoAgent`; `Forbidden` for a
/// tool or an agent the grants do not reach; else the tool attached,
/// after every connected tool attached before it — on an active agent
/// as well as an idle one, whose proxy serves the tool from now on and
/// tells the agent its tool list changed — and one attached already
/// left in its place. Nothing is connected: the connect opens at the
/// agent's first use.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::connected_standalone_tools::holds(&standing, Over::Attach) || !judge::agents::holds(&standing, AgentsOver::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NoTool);
    };
    let Some(agent) = agent_records::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NoAgent);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Attach, &tool).await?
        || !judge::agents::over(&standing, AgentsOver::Edit, &agent, agents::state(daemon, agent.id).await)
    {
        return Ok(Frame::Forbidden);
    }
    attachments::attach(&mut tx, tool.id, agent.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::ConnectedStandaloneTools);
    containers::attached_connected(daemon, &tool, &[agent.id]).await;
    Ok(Frame::Attached)
}
