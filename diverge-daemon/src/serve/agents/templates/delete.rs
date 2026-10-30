//! Deleting a agent template.

use std::sync::Arc;

use diverge_sdk::daemon::grant::agents_templates::Over;
use diverge_sdk::daemon::endpoints::agents::templates::delete::client::request;
use diverge_sdk::daemon::endpoints::agents::templates::delete::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::database;
use crate::containers::{ToolKey, services, tool_log};
use crate::store::tools::template_attachments;
use crate::store::{self, agents_templates, dependency_tools};
use super::in_use;

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
/// for a template the grants do not reach; `InUse` while some agent
/// was made from it; else the template deleted — every per-template
/// dependency tool it owns gone with its database scope and its every
/// table, each service among them retired, every tool attached to it
/// detached and told its hold changed, its row kept, so one made anew
/// is this one again, with fresh scopes and no attachments.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents_templates::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = agents_templates::by_id(&mut tx, &frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let held = store::in_use::agents_templates(&mut tx).await?;
    let used = in_use(&held, &record);
    if !judge::agents_templates::over(&standing, Over::Delete, &record, used) {
        return Ok(Frame::Forbidden);
    }
    if used {
        return Ok(Frame::InUse);
    }
    for tool in dependency_tools::of_owner_template(&mut tx, &record.id).await? {
        if let Some(container) = database::container_of_dependency_tool(&tool, None) {
            database::provision::drop(&mut tx, &database::role_of(&container)).await?;
        }
    }
    let removed = dependency_tools::delete_of_owner_template(&mut tx, &record.id).await?;
    let detached = template_attachments::clear_template(&mut tx, &record.id).await?;
    agents_templates::delete(&mut tx, &record.id).await?;
    tx.commit().await?;
    let gone: Vec<ToolKey> = removed.iter().map(|tool| ToolKey::Dependency(tool.id)).collect();
    services::template_deleted(daemon, removed).await;
    for key in gone {
        tool_log::forget(daemon, key).await;
    }
    for tool in detached {
        services::held(daemon, ToolKey::Record(tool)).await;
    }
    daemon.live.changed(Kind::AgentsTemplates);
    daemon.live.changed(Kind::DependencyTools);
    daemon.live.changed(Kind::StandaloneTools);
    Ok(Frame::Deleted)
}
