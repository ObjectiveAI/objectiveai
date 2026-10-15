//! Deleting a dependency template's tools.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::dependency_tools::delete::client::request;
use diverge_sdk::daemon::endpoints::dependency_tools::delete::server::response::Frame;
use diverge_sdk::daemon::grant::dependency_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::load;
use crate::containers::services;
use crate::daemon::{Daemon, Kind};
use crate::database;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents, dependencies, dependency_tools};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound` for a
/// template no tool is of and no body is on record for; `Forbidden`
/// for any tool of it the grants do not reach; `InUse` while any tool
/// of it runs, transient or a service; else every tool of the template
/// gone — each service among them retired, no agent attached to any,
/// each tool's database scope dropped with its every table — and the
/// template's body with them, so that a registration declaring it
/// anew makes it anew, with new scopes.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::dependency_tools::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let tools = dependency_tools::of_template(&mut tx, &frame.id, true).await?;
    if tools.is_empty() && dependencies::by_id(&mut tx, &frame.id).await?.is_none() {
        return Ok(Frame::NotFound);
    }
    let mut running = false;
    for tool in &tools {
        let Some(loaded) = load(&mut tx, daemon, tool).await? else {
            continue;
        };
        if !judge::dependency_tools::over(&standing, Over::Delete, &loaded.facts(tool)) {
            return Ok(Frame::Forbidden);
        }
        running |= loaded.run.is_some();
    }
    if running {
        return Ok(Frame::InUse);
    }
    for tool in &tools {
        let owner = match tool.owner_agent {
            Some(id) => agents::by_id(&mut tx, id, false).await?,
            None => None,
        };
        if let Some(container) = database::container_of_dependency_tool(tool, owner.as_ref()) {
            database::provision::drop(&mut tx, &database::role_of(&container)).await?;
        }
    }
    let removed = dependency_tools::delete_of_template(&mut tx, &frame.id).await?;
    dependencies::delete(&mut tx, &frame.id).await?;
    tx.commit().await?;
    services::template_deleted(daemon, removed).await;
    daemon.live.changed(Kind::DependencyTools);
    daemon.live.changed(Kind::StandaloneTools);
    Ok(Frame::Deleted)
}
