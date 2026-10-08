//! Downloading a file or a directory out of a tool's container.

use std::sync::Arc;

use diverge_sdk::daemon::download::Chunk;
use diverge_sdk::daemon::endpoints::tools::download::client::request;
use diverge_sdk::daemon::endpoints::tools::download::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store::{self, tools};
use crate::transfers::{Fail, Source};

/// Send what is at the path and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `download` grant at all; `NotFound`;
/// `Forbidden` for a tool the grants do not reach; the container
/// started, or the connected tool joined, for the operation, which
/// failing is the `Error`; `NotFound` for a path at which nothing is;
/// else every file at the path as chunks; the container released
/// after.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::tools::holds(&standing, Over::Download) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(tool) = tools::by_reference(&mut conn, &frame.tool, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let attached = agents_of(&mut conn, tool.id).await?;
    drop(conn);
    if !judge::tools::over(&standing, Over::Download, &tool, active(daemon, tool.id).await, &attached) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    if content::inside(&frame.path).is_err() {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    }
    let opened = match files::open_tool(daemon, &tool).await {
        Ok(opened) => opened,
        Err(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    };
    opened.touch();
    let source = Source::Opened(opened.clone());
    let outcome = files::download(scope, daemon, &source, &frame.path, &|scope, path, body| {
        Box::pin(async move {
            reply::reply(scope, &Frame::Chunk(Chunk { path: path.to_vec(), body })).await;
        })
    })
    .await;
    files::close(daemon, &opened).await;
    match outcome {
        Ok(()) => {}
        Err(Fail::NotFound) => reply::reply(scope, &Frame::NotFound).await,
        Err(fail) => reply::reply(scope, &Frame::Error(reply::failure(&fail))).await,
    }
    Ok(())
}
