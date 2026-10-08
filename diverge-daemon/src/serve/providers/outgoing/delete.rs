//! Forgetting an outgoing provider.

use diverge_sdk::daemon::endpoints::providers::outgoing::delete::client::request;
use diverge_sdk::daemon::endpoints::providers::outgoing::delete::server::response::Frame;
use diverge_sdk::daemon::grant::providers_outgoing::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_outgoing};

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
/// for a provider the grants do not reach; `InUse` while a container
/// is pinned to it or mounts or serves a volume of its; else the
/// provider forgotten, its dial ended and the connection it held
/// dropped, and its address free for an add.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_outgoing::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(provider) = providers_outgoing::by_address(&mut tx, &frame.address, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_provider_connected(&provider.identity()).await;
    if !judge::providers_outgoing::over(&standing, Over::Delete, &provider, connected) {
        return Ok(Frame::Forbidden);
    }
    if in_use(&provider) {
        return Ok(Frame::InUse);
    }
    providers_outgoing::delete(&mut tx, provider.id).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersOutgoing);
    daemon.live.stop_dial(&provider.address).await;
    Ok(Frame::Deleted)
}

/// Whether a container is pinned to the provider, or mounts or serves
/// a volume of its. No container exists yet, and no volume is served.
fn in_use(_: &providers_outgoing::Outgoing) -> bool {
    false
}
