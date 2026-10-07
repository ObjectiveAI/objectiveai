//! Uploading a file or a directory into a volume.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::volumes::upload::client::request;
use diverge_sdk::daemon::endpoints::volumes::upload::server::response::Frame;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Located, locate};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::files::{Into, What};
use crate::serve::{files, reply};
use crate::store;

/// Answer the upload and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let scope = Arc::new(scope);
    let answer = match serve(Arc::clone(&scope), frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `upload` grant at all; `NotFound`; `Error` for
/// a provider that could not be asked; `Forbidden` for a volume the
/// grants do not reach; `Held` while a running container has it or
/// an operation is on it — the upload takes it for its length; else
/// every file asked for on its own channel and landed at the path,
/// every parent made, and `Uploaded`; a file whose content ends in an
/// error is the `Error`, files landed before it staying.
async fn serve(scope: Arc<ScopeHandle>, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let (volume, path, what) = match &frame {
        request::Frame::File { volume, path } => (volume, path, What::File),
        request::Frame::Directory { volume, path, files } => (volume, path, What::Directory(files)),
    };
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::volumes::holds(&standing, Over::Upload) {
        return Ok(Frame::Forbidden);
    }
    let listed = match locate(&mut conn, daemon, volume).await? {
        Located::Volume(listed) => listed,
        Located::None => return Ok(Frame::NotFound),
        Located::Failed(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    drop(conn);
    if !judge::volumes::over(&standing, Over::Upload, &listed) {
        return Ok(Frame::Forbidden);
    }
    if !daemon.live.take_volume(volume).await {
        return Ok(Frame::Held);
    }
    let outcome = files::upload(scope, daemon, Into::Volume(volume), path, what).await;
    daemon.live.release_volume(volume).await;
    Ok(match outcome {
        Ok(()) => Frame::Uploaded,
        Err(error) => Frame::Error(reply::failure(&error)),
    })
}
