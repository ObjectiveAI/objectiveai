//! Replacing an outgoing provider's mode.

use diverge_sdk::daemon::endpoints::providers::outgoing::edit::client::request;
use diverge_sdk::daemon::endpoints::providers::outgoing::edit::server::response::Frame;
use diverge_sdk::daemon::grant::providers_outgoing::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_outgoing};

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
/// for a provider the grants do not reach; else the mode replaced
/// whole. A connection the daemon holds now is not dropped; the next
/// dial presents the new credential.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_outgoing::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(provider) = providers_outgoing::by_address(&mut tx, &frame.address, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_provider_connected(&provider.identity()).await;
    if !judge::providers_outgoing::over(&standing, Over::Edit, &provider, connected) {
        return Ok(Frame::Forbidden);
    }
    providers_outgoing::update_mode(&mut tx, provider.id, &frame.mode).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersOutgoing);
    Ok(Frame::Edited)
}
