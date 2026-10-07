//! Transferring a file or a directory out of a volume.

use std::sync::Arc;

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::volumes::transfer::client::request;
use diverge_sdk::daemon::endpoints::volumes::transfer::server::response::Frame;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Located, locate};
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::files::{Judged, judged};
use crate::serve::reply;
use crate::store;
use crate::transfers::{self, Fail, Source};

/// Answer the transfer and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `transfer` grant at all; `NotFound`; `Error`
/// for a provider that could not be asked; `Forbidden` for a volume
/// the grants do not reach, or a destination they do not; `NotFound`
/// for a path at which nothing is; `NoDestination`; `Held` for a
/// volume at either end that is held; else the copy, and
/// `Transferred` with the new resource's id when the destination was
/// one.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::volumes::holds(&standing, Over::Transfer) {
        return Ok(Frame::Forbidden);
    }
    let listed = match locate(&mut conn, daemon, &frame.volume).await? {
        Located::Volume(listed) => listed,
        Located::None => return Ok(Frame::NotFound),
        Located::Failed(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    if !judge::volumes::over(&standing, Over::Transfer, &listed) {
        return Ok(Frame::Forbidden);
    }
    if content::inside(&frame.path).is_err() {
        return Ok(Frame::NotFound);
    }
    match judged(&mut conn, daemon, &standing, &frame.to).await? {
        Judged::Allowed => {}
        Judged::Forbidden => return Ok(Frame::Forbidden),
        Judged::NoDestination => return Ok(Frame::NoDestination),
        Judged::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
    }
    drop(conn);
    let creator = Creator::Client(Client {
        identity: standing.identity.clone(),
    });
    Ok(match transfers::copy(daemon, Source::Volume(frame.volume), &frame.path, frame.to, creator).await {
        Ok(id) => Frame::Transferred(id),
        Err(Fail::NotFound) => Frame::NotFound,
        Err(Fail::NoDestination) => Frame::NoDestination,
        Err(Fail::Held) => Frame::Held,
        Err(fail) => Frame::Error(reply::failure(&fail)),
    })
}
