//! The tree of a directory in a volume, at rest.

use diverge_sdk::daemon::endpoints::volumes::filetree::client::request;
use diverge_sdk::daemon::endpoints::volumes::filetree::server::response::Frame;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Located, locate};
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;
use crate::volumes::{self, Found};

/// Answer the filetree and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `filetree` grant at all; `NotFound`; `Error`
/// for a provider that could not be asked; `Forbidden` for a volume
/// the grants do not reach; `Held` while a running container has it
/// or an operation is on it; `NotFound` for a path at which nothing
/// is; `NotDirectory` for a file; else the directory's tree, every
/// directory in it `changes` false — a volume at rest changes
/// nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::volumes::holds(&standing, Over::Filetree) {
        return Ok(Frame::Forbidden);
    }
    let listed = match locate(&mut conn, daemon, &frame.volume).await? {
        Located::Volume(listed) => listed,
        Located::None => return Ok(Frame::NotFound),
        Located::Failed(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    drop(conn);
    if !judge::volumes::over(&standing, Over::Filetree, &listed) {
        return Ok(Frame::Forbidden);
    }
    if volumes::held(daemon, &frame.volume).await {
        return Ok(Frame::Held);
    }
    if content::inside(&frame.path).is_err() {
        return Ok(Frame::NotFound);
    }
    Ok(match volumes::entry_at(daemon, &frame.volume, &frame.path).await {
        Ok(Found::Directory(mut nodes)) => {
            volumes::with_changes(&mut nodes, false);
            Frame::Tree(nodes)
        }
        Ok(Found::File(_)) => Frame::NotDirectory,
        Ok(Found::Missing) | Err(volumes::Fail::NotFound) => Frame::NotFound,
        Err(volumes::Fail::Held) => Frame::Held,
        Err(fail) => Frame::Error(reply::failure(&fail)),
    })
}
