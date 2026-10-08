//! Downloading a file or a directory out of an agent's container.

use std::sync::Arc;

use diverge_sdk::daemon::download::Chunk;
use diverge_sdk::daemon::endpoints::agents::download::client::request;
use diverge_sdk::daemon::endpoints::agents::download::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::active;
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store::{self, agents};
use crate::transfers::{Fail, Source};

/// Send what is at the path and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `download` grant at all; `NotFound`;
/// `Forbidden` for an agent the grants do not reach; the container
/// started if it was not up, which failing is the `Error`; `NotFound`
/// for a path at which nothing is; else every file at the path as
/// chunks, the daemon's mounts read from their sources.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents::holds(&standing, Over::Download) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Download, &agent, active(daemon, agent.id).await) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    if content::inside(&frame.path).is_err() {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    }
    let opened = match files::open_agent(daemon, &agent).await {
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
