//! Attaching a tool to an agent.

use diverge_sdk::daemon::endpoints::tools::attach::client::request;
use diverge_sdk::daemon::endpoints::tools::attach::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over as AgentsOver;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::agents;
use crate::serve::reply;
use crate::store::tools::attachments;
use crate::store::{self, agents as agent_records, tools};

/// Answer the attach and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `attach` grant over tools or no `edit` grant
/// over agents at all; `NoTool`; `NoAgent`; `Forbidden` for a tool or
/// an agent the grants do not reach; else the tool attached, after
/// every tool attached before it — on an active agent as well as an
/// idle one, and one attached already left in its place.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Attach) || !judge::agents::holds(&standing, AgentsOver::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NoTool);
    };
    let Some(agent) = agent_records::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NoAgent);
    };
    let attached = agents_of(&mut tx, tool.id).await?;
    if !judge::tools::over(&standing, Over::Attach, &tool, active(daemon, tool.id), &attached)
        || !judge::agents::over(&standing, AgentsOver::Edit, &agent, agents::active(daemon, agent.id))
    {
        return Ok(Frame::Forbidden);
    }
    attachments::attach(&mut tx, tool.id, agent.id).await?;
    tx.commit().await?;
    Ok(Frame::Attached)
}
