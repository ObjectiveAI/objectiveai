//! Downloading a file or a directory out of a running dependency
//! tool's container.

use std::sync::Arc;

use diverge_sdk::daemon::download::Chunk;
use diverge_sdk::daemon::endpoints::dependencies::download::client::request;
use diverge_sdk::daemon::endpoints::dependencies::download::server::response::Frame;
use diverge_sdk::daemon::grant::dependencies::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{reaches, resolve};
use crate::containers::Opened;
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store;
use crate::transfers::{Fail, Source};

/// Send what is at the path and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `download` grant at all; `NotFound` for a
/// template that is none, an owner it has no tool for, a tool that
/// does not run, or a path at which nothing is; `Forbidden` for a
/// template the grants do not reach; else every file at the path as
/// chunks, out of the run as it is. Nothing is started, and nothing
/// is released after.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::dependencies::holds(&standing, Over::Download) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some((record, run)) = resolve(&mut conn, daemon, &frame.dependency).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let reached = reaches(&mut conn, daemon, &standing, Over::Download, &record).await?;
    drop(conn);
    if !reached {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    if content::inside(&frame.path).is_err() {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    }
    let opened = Opened::Tool(run);
    opened.touch();
    let source = Source::Opened(opened);
    let outcome = files::download(scope, daemon, &source, &frame.path, &|scope, path, body| {
        Box::pin(async move {
            reply::reply(scope, &Frame::Chunk(Chunk { path: path.to_vec(), body })).await;
        })
    })
    .await;
    match outcome {
        Ok(()) => {}
        Err(Fail::NotFound) => reply::reply(scope, &Frame::NotFound).await,
        Err(fail) => reply::reply(scope, &Frame::Error(reply::failure(&fail))).await,
    }
    Ok(())
}
