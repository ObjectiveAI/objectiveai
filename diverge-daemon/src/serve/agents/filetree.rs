//! Watching an agent's container tree.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::filetree::client::request;
use diverge_sdk::daemon::endpoints::agents::filetree::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::active;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store::{self, agents};

/// Send the tree and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `filetree` grant at all; `NotFound`;
/// `Forbidden` for an agent the grants do not reach; the container
/// started if it was not up, which failing is the `Error`; else a
/// snapshot of the whole tree, the daemon's mounts spliced in, then
/// every change until the client cancels, the run ends, or the stream
/// fails. The watch touches nothing: it never keeps the container
/// up.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents::holds(&standing, Over::Filetree) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Filetree, &agent, active(daemon, agent.id).await) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let opened = match files::open_agent(daemon, &agent).await {
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
