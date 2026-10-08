//! Downloading a file or a directory out of a volume.

use diverge_sdk::daemon::download::Chunk;
use diverge_sdk::daemon::endpoints::volumes::download::client::request;
use diverge_sdk::daemon::endpoints::volumes::download::server::response::Frame;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Located, locate};
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store;
use crate::transfers::{Fail, Source};

/// Send what is at the path and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `download` grant at all; `NotFound`; `Error`
/// for a provider that could not be asked; `Forbidden` for a volume
/// the grants do not reach; `Held` while a running container has it
/// or an operation is on it — the download takes it for its length;
/// `NotFound` for a path at which nothing is; else every file at the
/// path as chunks.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::volumes::holds(&standing, Over::Download) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let listed = match locate(&mut conn, daemon, &frame.volume).await? {
        Located::Volume(listed) => listed,
        Located::None => {
            reply::reply(scope, &Frame::NotFound).await;
            return Ok(());
        }
        Located::Failed(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    };
    drop(conn);
    if !judge::volumes::over(&standing, Over::Download, &listed) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    if content::inside(&frame.path).is_err() {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    }
    if !daemon.live.take_volume(&frame.volume).await {
        reply::reply(scope, &Frame::Held).await;
        return Ok(());
    }
    let source = Source::Volume(frame.volume.clone());
    let outcome = files::download(scope, daemon, &source, &frame.path, &|scope, path, body| {
        Box::pin(async move {
            reply::reply(scope, &Frame::Chunk(Chunk { path: path.to_vec(), body })).await;
        })
    })
    .await;
    daemon.live.release_volume(&frame.volume).await;
    match outcome {
        Ok(()) => {}
        Err(Fail::NotFound) => reply::reply(scope, &Frame::NotFound).await,
        Err(Fail::Held) => reply::reply(scope, &Frame::Held).await,
        Err(fail) => reply::reply(scope, &Frame::Error(reply::failure(&fail))).await,
    }
    Ok(())
}
