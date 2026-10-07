//! Watching a tool's container tree.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::tools::filetree::client::request;
use diverge_sdk::daemon::endpoints::tools::filetree::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store::{self, tools};

/// Send the tree and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `filetree` grant at all; `NotFound`;
/// `Forbidden` for a tool the grants do not reach; the container
/// started, or the connected tool joined, for the watch, which
/// failing is the `Error`; else a snapshot of the whole tree, the
/// daemon's mounts spliced in, then every change until the client
/// cancels, the run ends, or the stream fails; the container released
/// after. The watch touches nothing: it never keeps the container up
/// on its own.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::tools::holds(&standing, Over::Filetree) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(tool) = tools::by_reference(&mut conn, &frame.tool, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let attached = agents_of(&mut conn, tool.id).await?;
    drop(conn);
    if !judge::tools::over(&standing, Over::Filetree, &tool, active(daemon, tool.id).await, &attached) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let opened = match files::open_tool(daemon, &tool).await {
        Ok(opened) => opened,
        Err(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    };
    let outcome = files::watch(scope, &opened, &|scope, frame| {
        Box::pin(async move {
            reply::reply(scope, &Frame::Filetree(frame)).await;
        })
    })
    .await;
    files::close(daemon, &opened).await;
    if let Err(error) = outcome {
        reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
    }
    Ok(())
}
