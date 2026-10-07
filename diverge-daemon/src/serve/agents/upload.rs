//! Uploading a file or a directory into an agent's container.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::upload::client::request;
use diverge_sdk::daemon::endpoints::agents::upload::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::active;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::files::{Into, What};
use crate::serve::{files, reply};
use crate::store::{self, agents};

/// Answer the upload and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let scope = Arc::new(scope);
    let answer = match serve(Arc::clone(&scope), frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `upload` grant at all; `NotFound`; `Forbidden`
/// for an agent the grants do not reach; the container started if it
/// was not up, which failing is the `Error`; else every file asked
/// for on its own channel and landed at the path, every parent made,
/// the daemon's mounts written through their sources, and
/// `Uploaded`; a file whose content ends in an error is the `Error`,
/// files landed before it staying.
async fn serve(scope: Arc<ScopeHandle>, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let (reference, path, what) = match &frame {
        request::Frame::File { agent, path } => (agent, path, What::File),
        request::Frame::Directory { agent, path, files } => (agent, path, What::Directory(files)),
    };
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents::holds(&standing, Over::Upload) {
        return Ok(Frame::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut conn, reference, false).await? else {
        return Ok(Frame::NotFound);
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Upload, &agent, active(daemon, agent.id).await) {
        return Ok(Frame::Forbidden);
    }
    let opened = match files::open_agent(daemon, &agent).await {
        Ok(opened) => opened,
        Err(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    let outcome = files::upload(scope, daemon, Into::Opened(&opened), path, what).await;
    files::close(daemon, &opened).await;
    Ok(match outcome {
        Ok(()) => Frame::Uploaded,
        Err(error) => Frame::Error(reply::failure(&error)),
    })
}
