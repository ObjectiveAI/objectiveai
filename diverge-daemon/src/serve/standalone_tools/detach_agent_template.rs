//! Detaching a tool from every agent of a template.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::standalone_tools::detach_agent_template::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::detach_agent_template::server::response::Frame;
use diverge_sdk::daemon::grant::agents_templates::Over as TemplatesOver;
use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::containers::{self, ToolKey, services};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::agents::templates::in_use;
use crate::serve::reply;
use crate::store::tools::template_attachments;
use crate::store::{self, agents_templates, tools};

/// Answer the detach and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `detach` grant over tools or no `attach` grant
/// over agent templates at all; `NoTool`; `NoTemplate`; `Forbidden`
/// for a tool or a template the grants do not reach; else the tool
/// detached from the template, one not attached nothing to take back:
/// every agent of the template loses it at this instant, unless
/// attached to it directly as well — a running one's proxy serves it
/// no more and tells the agent its tool list changed, a call in
/// flight answered still. A service is told its hold changed.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::standalone_tools::holds(&standing, Over::Detach) || !judge::agents_templates::holds(&standing, TemplatesOver::Attach) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NoTool);
    };
    let Some(record) = agents_templates::by_id(&mut tx, &frame.template, true).await? else {
        return Ok(Frame::NoTemplate);
    };
    let held = store::in_use::agents_templates(&mut tx).await?;
    if !reaches(&mut tx, daemon, &standing, Over::Detach, &tool).await?
        || !judge::agents_templates::over(&standing, TemplatesOver::Attach, &record, in_use(&held, &record))
    {
        return Ok(Frame::Forbidden);
    }
    template_attachments::detach(&mut tx, tool.id, &record.id).await?;
    let agents = tools::agents_of_template(&mut tx, &record.id).await?;
    let keeps = tools::attached_agents(&mut tx, tool.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::AgentsTemplates);
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::StandaloneTools);
    services::held(daemon, ToolKey::Record(tool.id)).await;
    containers::detached(daemon, tool.id, &agents, &keeps).await;
    Ok(Frame::Detached)
}
