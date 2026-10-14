//! Deleting a tool.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::standalone_tools::delete::client::request;
use diverge_sdk::daemon::endpoints::standalone_tools::delete::server::response::Frame;
use diverge_sdk::daemon::grant::standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{agents_of, reaches, resolve};
use crate::containers::{Key, ToolKey, services};
use crate::daemon::{Daemon, Kind};
use crate::database;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tools};

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
/// for a tool the grants do not reach; `Attached` while it is
/// attached to any agent; else the tool gone — its own database scope
/// dropped with its every table, its name free, and, a service, its
/// keeping ended: held by no agent, it may still be held by a connect
/// or be waiting on its clock, and that run is stopped with it. A
/// transient tool attached nowhere runs nowhere, so nothing is
/// stopped.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::standalone_tools::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Delete, &tool).await? {
        return Ok(Frame::Forbidden);
    }
    let attached = agents_of(&mut tx, tool.id).await?;
    if !attached.is_empty() {
        return Ok(Frame::Attached);
    }
    tools::delete(&mut tx, tool.id).await?;
    let role = database::role_of(&database::container_of_tool(&tool));
    database::provision::drop(&mut tx, &role).await?;
    tx.commit().await?;
    services::tool_deleted(daemon, tool.id).await;
    daemon.live.forget_scope(Key::Tool(ToolKey::Record(tool.id))).await;
    daemon.live.changed(Kind::StandaloneTools);
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Deleted)
}
