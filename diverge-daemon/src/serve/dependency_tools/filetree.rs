//! Watching a running dependency tool's container tree.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::dependency_tools::filetree::client::request;
use diverge_sdk::daemon::endpoints::dependency_tools::filetree::server::response::Frame;
use diverge_sdk::daemon::grant::dependency_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::containers::Opened;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store;

/// Send the tree and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `filetree` grant at all; `NotFound` for a
/// template that is none, an owner it has no tool under, or a tool
/// that does not run; `Forbidden` for a tool the grants do not reach;
/// else a snapshot of the whole tree, the daemon's mounts spliced in,
/// then every change until the client cancels, the run ends, or the
/// stream fails. Nothing is started, nothing is released after, and
/// the watch touches nothing: it never keeps the container up on its
/// own.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::dependency_tools::holds(&standing, Over::Filetree) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some((tool, Some(run))) = resolve(&mut conn, daemon, &frame.dependency, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let reached = reaches(&mut conn, daemon, &standing, Over::Filetree, &tool).await?;
    drop(conn);
    if !reached {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let opened = Opened::Tool(run, None);
    let outcome = files::watch(scope, &opened, &|scope, frame| {
        Box::pin(async move {
            reply::reply(scope, &Frame::Filetree(frame)).await;
        })
    })
    .await;
    if let Err(error) = outcome {
        reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
    }
    Ok(())
}
