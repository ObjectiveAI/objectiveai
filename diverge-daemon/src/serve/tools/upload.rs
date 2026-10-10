//! Uploading a file or a directory into a tool's container.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::tools::upload::client::request;
use diverge_sdk::daemon::endpoints::tools::upload::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Found, READ_ONLY, reaches, resolve};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::files::{Into, What};
use crate::serve::{files, reply};
use crate::store;

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
/// for a tool the grants do not reach; the error for a dependency,
/// whose files are read and never written from here; the container
/// started, or the
/// connected tool joined, for the operation, which failing is the
/// `Error`; else every file asked for on its own channel and landed
/// at the path, every parent made, and `Uploaded`; a file whose
/// content ends in an error is the `Error`, files landed before it
/// staying; the container released after.
async fn serve(scope: Arc<ScopeHandle>, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let (reference, path, what) = match &frame {
        request::Frame::File { tool, path } => (tool, path, What::File),
        request::Frame::Directory { tool, path, files } => (tool, path, What::Directory(files)),
    };
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Upload) {
        return Ok(Frame::Forbidden);
    }
    let Some(found) = resolve(&mut conn, daemon, reference, false).await? else {
        return Ok(Frame::NotFound);
    };
    let reached = reaches(&mut conn, daemon, &standing, Over::Upload, &found).await?;
    drop(conn);
    if !reached {
        return Ok(Frame::Forbidden);
    }
    let Found::Record(tool) = found else {
        return Ok(Frame::Error(reply::failure(&READ_ONLY)));
    };
    let opened = match files::open_tool(daemon, &tool).await {
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
