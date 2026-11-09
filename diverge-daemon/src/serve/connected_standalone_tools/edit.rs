//! Editing a connected tool: a patch to its name, the one member an
//! edit changes.

use diverge_sdk::daemon::endpoints::connected_standalone_tools::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::connected_standalone_tools::edit::server::response::Frame;
use diverge_sdk::daemon::grant::connected_standalone_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner;
use crate::serve::reply;
use crate::store::{self, connected_standalone_tools};

/// Answer the edit and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. No standing, or no `edit` grant at all, is `Forbidden`.
/// 2. A tool the reference names none of is `NotFound`; one the grants
///    do not reach is `Forbidden`.
/// 3. A patch that could not be applied, or a result that is not a
///    connected tool's editable members — its name, and nothing else —
///    is the `patch` error.
/// 4. A name a standalone or another connected tool has is `InUse`,
///    as [`tool_names`](crate::store::tool_names) states.
/// 5. Else the tool is called as the result states. Whether its connect
///    is held is nothing to the edit: an agent running serves it under
///    the prefix it was given, until its next run.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::connected_standalone_tools::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Edit, &tool).await? {
        return Ok(Frame::Forbidden);
    }
    let current = Editable { name: tool.name.clone() };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    if result == current {
        return Ok(Frame::Edited);
    }
    if let connected_standalone_tools::Renamed::InUse = connected_standalone_tools::rename(&mut tx, tool.id, current.name.as_deref(), result.name.as_deref()).await? {
        return Ok(Frame::InUse);
    }
    tx.commit().await?;
    daemon.live.changed(Kind::ConnectedStandaloneTools);
    daemon.live.changed(Kind::Agents);
    daemon.live.changed(Kind::AgentsTemplates);
    Ok(Frame::Edited)
}
