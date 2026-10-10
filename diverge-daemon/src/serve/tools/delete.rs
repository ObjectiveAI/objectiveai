//! Deleting a tool.

use diverge_sdk::daemon::endpoints::tools::delete::client::request;
use diverge_sdk::daemon::endpoints::tools::delete::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Found, READ_ONLY, agents_of, reaches, resolve};
use crate::containers::{Key, ToolKey};
use crate::daemon::{Daemon, Kind};
use crate::database;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tools};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden`
/// for a tool the grants do not reach; the error for a dependency,
/// which is its agent's and goes with it; `Attached` while it is
/// attached to any agent; else the tool gone — its database scope
/// dropped with its every table, its admissions with it, its name
/// free. A tool attached nowhere runs nowhere, so nothing is stopped.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(found) = resolve(&mut tx, daemon, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Delete, &found).await? {
        return Ok(Frame::Forbidden);
    }
    let Found::Record(tool) = found else {
        return Ok(Frame::Error(reply::failure(&READ_ONLY)));
    };
    let attached = agents_of(&mut tx, tool.id).await?;
    if !attached.is_empty() {
        return Ok(Frame::Attached);
    }
    tools::delete(&mut tx, tool.id).await?;
    database::provision::drop(&mut tx, &database::role_of(&database::container_of_tool(&tool))).await?;
    tx.commit().await?;
    daemon.live.forget_scope(Key::Tool(ToolKey::Record(tool.id))).await;
    daemon.live.changed(Kind::Tools);
    daemon.live.changed(Kind::Volumes);
    Ok(Frame::Deleted)
}
