//! Attaching a tool to an agent.

use diverge_sdk::daemon::endpoints::tools::attach::client::request;
use diverge_sdk::daemon::endpoints::tools::attach::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over as AgentsOver;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Found, READ_ONLY, reaches, resolve};
use crate::containers;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::agents;
use crate::serve::reply;
use crate::store::tools::attachments;
use crate::store::{self, agents as agent_records};

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
/// an agent the grants do not reach; the error for a dependency,
/// which is served to its own agent and attached to none; else the
/// tool attached, after
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
    let Some(found) = resolve(&mut tx, daemon, &frame.tool, true).await? else {
        return Ok(Frame::NoTool);
    };
    let Some(agent) = agent_records::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NoAgent);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Attach, &found).await?
        || !judge::agents::over(&standing, AgentsOver::Edit, &agent, agents::active(daemon, agent.id).await)
    {
        return Ok(Frame::Forbidden);
    }
    let Found::Record(tool) = found else {
        return Ok(Frame::Error(reply::failure(&READ_ONLY)));
    };
    attachments::attach(&mut tx, tool.id, agent.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::Tools);
    if let Some(run) = daemon.live.agent_run(agent.id).await {
        let name = containers::serve_name(&tool);
        run.served.lock().await.insert(tool, &name);
    }
    Ok(Frame::Attached)
}
