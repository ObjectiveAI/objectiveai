//! Attaching a tool to every agent of a template.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::standalone_tools::attach_agent_template::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::attach_agent_template::server::response::Frame;
use diverge_sdk::daemon::grant::agents_templates::Over as TemplatesOver;
use diverge_sdk::daemon::endpoints::standalone_tools::templates::Mode;
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

/// Answer the attach and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `attach` grant over tools or over agent
/// templates at all; `NoTool`; `NoTemplate`; `Forbidden` for a tool
/// or a template the grants do not reach; else the tool attached to
/// the template, after every tool attached to it before, and one
/// attached already left in its place: every agent made from the
/// template has it from now on — a running one's proxy serves it from
/// this instant and tells the agent its tool list changed. A tool
/// whose template's mode is a service is kept from the attach on.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::standalone_tools::holds(&standing, Over::Attach) || !judge::agents_templates::holds(&standing, TemplatesOver::Attach) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NoTool);
    };
    let Some(record) = agents_templates::by_id(&mut tx, &frame.template, true).await? else {
        return Ok(Frame::NoTemplate);
    };
    let held = store::in_use::agents_templates(&mut tx).await?;
    if !reaches(&mut tx, daemon, &standing, Over::Attach, &tool).await?
        || !judge::agents_templates::over(&standing, TemplatesOver::Attach, &record, in_use(&held, &record))
    {
        return Ok(Frame::Forbidden);
    }
    template_attachments::attach(&mut tx, tool.id, &record.id).await?;
    let service = tools::mode(&mut tx, &tool).await? == Some(Mode::Service);
    let agents = tools::agents_of_template(&mut tx, &record.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::AgentsTemplates);
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::StandaloneTools);
    if service {
        services::keep(daemon, ToolKey::Record(tool.id)).await;
        services::held(daemon, ToolKey::Record(tool.id)).await;
    }
    containers::attached(daemon, &tool, &agents).await;
    Ok(Frame::Attached)
}
