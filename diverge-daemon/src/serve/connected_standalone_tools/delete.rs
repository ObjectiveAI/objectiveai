//! Deleting a connected tool.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::connected_standalone_tools::delete::client::request;
use diverge_sdk::daemon::endpoints::connected_standalone_tools::delete::server::response::Frame;
use diverge_sdk::daemon::grant::connected_standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::containers::lifecycle::End;
use crate::containers::{ToolKey, tool_log};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, connected_standalone_tools};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. No standing, or no `delete` grant at all, is `Forbidden`.
/// 2. A tool the reference names none of is `NotFound`; one the grants
///    do not reach is `Forbidden`.
/// 3. A tool attached to any agent or any agent template is `Attached`.
/// 4. Else the record is gone, its name free, its log removed, and the
///    connect this daemon holds on it, if any — kept by its idle clock
///    since its last use — let go now.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::connected_standalone_tools::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Delete, &tool).await? {
        return Ok(Frame::Forbidden);
    }
    let attached = !connected_standalone_tools::attachments::of_tool(&mut tx, tool.id).await?.is_empty()
        || !connected_standalone_tools::template_attachments::of_tool(&mut tx, tool.id).await?.is_empty();
    if attached {
        return Ok(Frame::Attached);
    }
    connected_standalone_tools::delete(&mut tx, tool.id).await?;
    tx.commit().await?;
    let key = ToolKey::Connected(tool.id);
    if let Some(run) = daemon.live.tool_run(key).await {
        run.end_as(End::Expected).await;
        run.handle.stop().await;
    }
    tool_log::forget(daemon, key).await;
    daemon.live.changed(Kind::ConnectedStandaloneTools);
    Ok(Frame::Deleted)
}
