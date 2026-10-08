//! Transferring a file or a directory out of a tool's container.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::tools::transfer::client::request;
use diverge_sdk::daemon::endpoints::tools::transfer::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::files::{Judged, judged};
use crate::serve::{files, reply};
use crate::store::{self, tools};
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

/// `Forbidden` with no `transfer` grant at all; `NotFound`;
/// `Forbidden` for a tool the grants do not reach, or a destination
/// they do not; `NoDestination`; the container started, or the
/// connected tool joined, for the operation, which failing is the
/// `Error`; `NotFound` for a path at which nothing is; `Held` for a
/// destination volume that is held; else the copy — through the
/// provider when both ends run on it — and `Transferred`; the
/// container released after.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Transfer) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut conn, &frame.tool, false).await? else {
        return Ok(Frame::NotFound);
    };
    let attached = agents_of(&mut conn, tool.id).await?;
    if !judge::tools::over(&standing, Over::Transfer, &tool, active(daemon, tool.id).await, &attached) {
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
    let opened = match files::open_tool(daemon, &tool).await {
        Ok(opened) => opened,
        Err(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    opened.touch();
    let outcome = transfers::copy(daemon, Source::Opened(opened.clone()), &frame.path, frame.to).await;
    files::close(daemon, &opened).await;
    Ok(match outcome {
        Ok(()) => Frame::Transferred,
        Err(Fail::NotFound) => Frame::NotFound,
        Err(Fail::NoDestination) => Frame::NoDestination,
        Err(Fail::Held) => Frame::Held,
        Err(fail) => Frame::Error(reply::failure(&fail)),
    })
}
