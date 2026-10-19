//! Editing a volume: a patch to its size and its mode.

use diverge_sdk::daemon::endpoints::volumes::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::volumes::edit::server::response::Frame;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::provider::endpoints::volumes::edit::client::request::Change;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Located, locate};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::inner;
use crate::serve::reply;
use crate::store;
use crate::volumes::{self, Edited};

/// Answer the edit and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `edit` grant at all; `NotFound`; `Error` for a
/// provider that could not be asked; `Forbidden` for a volume the
/// grants do not reach; the `patch` error for a patch that could not
/// be applied or a result that is not a volume's size and mode;
/// `Edited` with nothing changed; `Held` while a running container
/// has it or an operation is on it, or the provider says it is
/// mounted; `InsufficientCapacity` and `ContentTooLarge` as the
/// provider says; else `Edited`.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::volumes::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let listed = match locate(&mut conn, daemon, &frame.volume).await? {
        Located::Volume(listed) => listed,
        Located::None => return Ok(Frame::NotFound),
        Located::Failed(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    drop(conn);
    if !judge::volumes::over(&standing, Over::Edit, &listed) {
        return Ok(Frame::Forbidden);
    }
    let current = Editable {
        bytes: listed.volume.bytes,
        mode: listed.volume.mode,
    };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    let change = match (result.bytes != current.bytes, result.mode != current.mode) {
        (false, false) => return Ok(Frame::Edited),
        (true, false) => Change::Bytes(result.bytes),
        (false, true) => Change::Mode(result.mode),
        (true, true) => Change::Both {
            bytes: result.bytes,
            mode: result.mode,
        },
    };
    if volumes::held(daemon, &frame.volume).await {
        return Ok(Frame::Held);
    }
    Ok(match volumes::edit(daemon, &frame.volume, change).await {
        Ok(Edited::Edited) => Frame::Edited,
        Ok(Edited::InsufficientCapacity) => Frame::InsufficientCapacity,
        Ok(Edited::ContentTooLarge) => Frame::ContentTooLarge,
        Err(volumes::Fail::Held) => Frame::Held,
        Err(volumes::Fail::NotFound) => Frame::NotFound,
        Err(fail) => Frame::Error(fail.error()),
    })
}
