//! Changing a remote daemon's record.

use diverge_sdk::daemon::endpoints::daemons::edit::client::request;
use diverge_sdk::daemon::endpoints::daemons::edit::server::response::Frame;
use diverge_sdk::daemon::grant::daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the request and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `edit` grant at all; `NotFound`; `Forbidden` for
/// a record the grants do not reach; the `Error` for our identity at
/// another issuer than the remote's, as the record ends up; else the
/// record changed, its connection — if one is held — dropped, so the
/// next use connects anew, and the lists told.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::daemons::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(mut record) = store::daemons::by_name(&mut tx, &frame.name, true).await? else {
        return Ok(Frame::NotFound);
    };
    let reported = record.report(daemon.live.is_daemon_connected(&record.name).await);
    if !judge::daemons::over(&standing, Over::Edit, &reported) {
        return Ok(Frame::Forbidden);
    }
    if let Some(description) = frame.description {
        record.description = description;
    }
    if let Some(remote) = frame.daemon {
        record.daemon = remote;
    }
    if let Some(identity) = frame.identity {
        record.identity = identity;
    }
    if let Some(providers) = frame.providers {
        record.routes.providers = providers;
    }
    if let Some(ips) = frame.ips {
        record.routes.ips = ips;
    }
    if let Some(addresses) = frame.addresses {
        record.routes.addresses = addresses;
    }
    if record.identity.issuer != record.daemon.issuer {
        return Ok(Frame::Error(reply::failure(&"our identity must be at the remote's issuer: the handshake between the two speaks one")));
    }
    store::daemons::edit(&mut tx, &record).await?;
    tx.commit().await?;
    if let Some(peer) = daemon.live.daemon_peer(&record.name).await {
        daemon.live.remove_peer(&peer).await;
    }
    daemon.live.changed(Kind::Daemons);
    Ok(Frame::Edited)
}
