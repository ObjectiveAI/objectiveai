//! Deleting a tool.

use diverge_sdk::daemon::endpoints::tools::delete::client::request;
use diverge_sdk::daemon::endpoints::tools::delete::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::daemon::Daemon;
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
/// for a tool the grants do not reach; `Attached` while it is attached
/// to any agent; else the tool gone — its admissions and the routes
/// that named it with it, its name free. A tool attached nowhere runs
/// nowhere, so nothing is stopped.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    let attached = agents_of(&mut tx, tool.id).await?;
    if !judge::tools::over(&standing, Over::Delete, &tool, active(daemon, tool.id), &attached) {
        return Ok(Frame::Forbidden);
    }
    if !attached.is_empty() {
        return Ok(Frame::Attached);
    }
    tools::delete(&mut tx, tool.id).await?;
    tx.commit().await?;
    Ok(Frame::Deleted)
}
