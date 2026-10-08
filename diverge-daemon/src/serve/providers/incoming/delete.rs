//! Taking an incoming credential out.

use diverge_sdk::daemon::endpoints::providers::incoming::delete::client::request;
use diverge_sdk::daemon::endpoints::providers::incoming::delete::server::response::Frame;
use diverge_sdk::daemon::grant::providers_incoming::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_incoming};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden`
/// for a credential the grants do not reach; `InUse` while a provider
/// is connected through it; else the credential gone, and its key
/// admitting nothing from then on.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_incoming::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(credential) = providers_incoming::by_identity(&mut tx, &frame.identity, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_provider_connected(&credential.provider()).await;
    if !judge::providers_incoming::over(&standing, Over::Delete, &credential, connected) {
        return Ok(Frame::Forbidden);
    }
    if connected {
        return Ok(Frame::InUse);
    }
    providers_incoming::delete(&mut tx, credential.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersIncoming);
    Ok(Frame::Deleted)
}
