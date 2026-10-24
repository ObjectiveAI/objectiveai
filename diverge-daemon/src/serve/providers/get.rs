//! Getting one provider.

use diverge_sdk::daemon::endpoints::providers::get::client::request;
use diverge_sdk::daemon::endpoints::providers::get::server::response::Frame;
use diverge_sdk::daemon::endpoints::providers::list::server::response::Provider;
use diverge_sdk::daemon::grant::providers::Action;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `get` grant; `NotFound` for an identity the
/// configuration does not accept; else the provider as a list reports
/// it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers::holds(&standing, Action::Get) {
        return Ok(Frame::Forbidden);
    }
    if !daemon.providers.accepted.contains(&frame.identity) {
        return Ok(Frame::NotFound);
    }
    Ok(Frame::Found(report(daemon, &frame.identity).await))
}

/// The provider of `identity` as a get and a list report it: the
/// address a dial last reached it at, where the daemon stands with it
/// now, and its latest capacity, while it is connected.
pub async fn report(daemon: &Daemon, identity: &Identity) -> Provider {
    let status = daemon.live.provider_status(identity).await;
    let capacity = if status.connected() { daemon.live.capacity(identity).map(|frame| (*frame).clone()) } else { None };
    Provider {
        identity: identity.clone(),
        address: daemon.live.address(identity).await,
        status,
        capacity,
    }
}
