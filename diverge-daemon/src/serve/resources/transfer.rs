//! Transferring a resource, or a part of one, somewhere.

use std::sync::Arc;

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::resources::transfer::client::request;
use diverge_sdk::daemon::endpoints::resources::transfer::server::response::Frame;
use diverge_sdk::daemon::grant::resources::Over;
use diverge_sdk::daemon::transfer::Destination;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::in_use;
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::files::{Judged, judged};
use crate::serve::reply;
use crate::store::{self, resources};
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

/// `Forbidden` with no `transfer` grant at all; `NotFound` for a
/// resource not held, or a path at which nothing is; `Forbidden` for
/// a resource the grants do not reach, or a destination they do not;
/// `IntoResource` for a resource destination, which a resource is
/// never copied into; `NoDestination`; `Held` for a destination
/// volume that is held; else the copy, the file landing whole or the
/// directory's files one by one, and `Transferred(None)`.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::resources::holds(&standing, Over::Transfer) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = resources::by_id(&mut conn, &frame.resource, false).await? else {
        return Ok(Frame::NotFound);
    };
    let held = store::in_use::resources(&mut conn).await?;
    if !judge::resources::over(&standing, Over::Transfer, &record, in_use(&held, &record)) {
        return Ok(Frame::Forbidden);
    }
    if content::inside(&frame.path).is_err() {
        return Ok(Frame::NotFound);
    }
    let root = content::held(&daemon.resources, &record.id);
    if content::at(&root, &frame.path).await.is_none() {
        return Ok(Frame::NotFound);
    }
    if matches!(frame.to, Destination::Resource { .. }) {
        return Ok(Frame::IntoResource);
    }
    match judged(&mut conn, daemon, &standing, &frame.to).await? {
        Judged::Allowed => {}
        Judged::Forbidden => return Ok(Frame::Forbidden),
        Judged::NoDestination => return Ok(Frame::NoDestination),
        Judged::Error(error) => return Ok(Frame::Error(reply::failure(&error))),
    }
    drop(conn);
    let source = Source::Resource { root, kind: record.kind };
    let creator = Creator::Client(Client {
        identity: standing.identity.clone(),
    });
    Ok(match transfers::copy(daemon, source, &frame.path, frame.to, creator).await {
        Ok(id) => Frame::Transferred(id),
        Err(Fail::NotFound) => Frame::NotFound,
        Err(Fail::NoDestination) => Frame::NoDestination,
        Err(Fail::Held) => Frame::Held,
        Err(fail) => Frame::Error(reply::failure(&fail)),
    })
}
