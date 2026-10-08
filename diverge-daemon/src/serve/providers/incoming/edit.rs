//! Replacing an incoming credential.

use diverge_sdk::daemon::endpoints::providers::incoming::edit::client::request;
use diverge_sdk::daemon::endpoints::providers::incoming::edit::server::response::Frame;
use diverge_sdk::daemon::grant::providers_incoming::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, key};
use crate::serve::reply;
use crate::store::{self, providers_incoming};

/// Answer the edit and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `edit` grant at all; `NotFound`; `Forbidden`
/// for a credential the grants do not reach; `InUse` for a new
/// identity another credential names; else the credential replaced
/// whole, with a new key answered here and never again, the old one
/// admitting nothing. A connection a provider holds through it now is
/// ended by the daemon once the edit is committed — its slot given
/// back as it closes — and the next one is judged by the new key.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_incoming::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(credential) = providers_incoming::by_identity(&mut tx, &frame.identity, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_provider_connected(&credential.provider()).await;
    if !judge::providers_incoming::over(&standing, Over::Edit, &credential, connected) {
        return Ok(Frame::Forbidden);
    }
    let minted = key::mint();
    let columns = providers_incoming::Columns {
        identity: frame.set.identity,
        address: frame.set.address,
        key_hash: key::hash(&minted),
    };
    if let providers_incoming::Updated::InUse = providers_incoming::update(&mut tx, credential.id, &columns).await? {
        return Ok(Frame::InUse);
    }
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersIncoming);
    daemon.live.evict_provider(&credential.provider()).await;
    Ok(Frame::Edited(minted))
}
