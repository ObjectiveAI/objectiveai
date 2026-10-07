//! Deleting a volume.

use diverge_sdk::daemon::endpoints::volumes::delete::client::request;
use diverge_sdk::daemon::endpoints::volumes::delete::server::response::Frame;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Located, locate};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;
use crate::volumes::{self, Dropped};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Error` for
/// a provider that could not be asked; `Forbidden` for a volume the
/// grants do not reach; `InUse` while some record names it in its
/// mounts, running or not, or an operation is on it, or the provider
/// says a container mounts it; else `Deleted`.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::volumes::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let listed = match locate(&mut conn, daemon, &frame.volume).await? {
        Located::Volume(listed) => listed,
        Located::None => return Ok(Frame::NotFound),
        Located::Failed(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    if !judge::volumes::over(&standing, Over::Delete, &listed) {
        return Ok(Frame::Forbidden);
    }
    if volumes::in_use(&mut conn, daemon, &frame.volume).await? {
        return Ok(Frame::InUse);
    }
    drop(conn);
    Ok(match volumes::delete(daemon, &frame.volume).await {
        Ok(Dropped::Deleted) => Frame::Deleted,
        Ok(Dropped::Mounted) | Err(volumes::Fail::Held) => Frame::InUse,
        Err(volumes::Fail::NotFound) => Frame::NotFound,
        Err(fail) => Frame::Error(reply::failure(&fail)),
    })
}
