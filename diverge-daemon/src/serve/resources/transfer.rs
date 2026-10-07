//! Transferring a resource, or a part of one, somewhere.

use diverge_sdk::daemon::endpoints::resources::transfer::client::request;
use diverge_sdk::daemon::endpoints::resources::transfer::server::response::Frame;
use diverge_sdk::daemon::grant::resources::Over;
use diverge_sdk::daemon::transfer::Destination;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::in_use;
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, resources};

/// Answer the transfer and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `transfer` grant at all; `NotFound` for a
/// resource not held, or a path at which nothing is; `Forbidden` for a
/// resource the grants do not reach; `IntoResource` for a resource
/// destination, which a resource is never copied into; and
/// `NoDestination` for an agent, a tool or a volume, since none exists
/// yet — the landing comes with them.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
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
    drop(conn);
    if !judge::resources::over(&standing, Over::Transfer, &record, in_use(&record)) {
        return Ok(Frame::Forbidden);
    }
    if content::inside(&frame.path).is_err() {
        return Ok(Frame::NotFound);
    }
    let root = content::held(&daemon.resources, &record.id);
    if content::at(&root, &frame.path).await.is_none() {
        return Ok(Frame::NotFound);
    }
    Ok(match frame.to {
        Destination::Resource { .. } => Frame::IntoResource,
        Destination::Agent { .. } | Destination::Tool { .. } | Destination::Volume { .. } => Frame::NoDestination,
    })
}
