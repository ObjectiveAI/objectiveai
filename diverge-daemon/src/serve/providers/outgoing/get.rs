//! Getting one outgoing provider.

use diverge_sdk::daemon::endpoints::providers::outgoing::get::client::request;
use diverge_sdk::daemon::endpoints::providers::outgoing::get::server::response::Frame;
use diverge_sdk::daemon::grant::providers_outgoing::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_outgoing};

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `get` grant at all; `NotFound`; `Forbidden`
/// for a provider the grants do not reach; else the provider as a
/// list reports it, never its credential.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_outgoing::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(provider) = providers_outgoing::by_address(&mut conn, &frame.address, false).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_provider_connected(&provider.identity()).await;
    if !judge::providers_outgoing::over(&standing, Over::Get, &provider, connected) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(provider.report(connected)))
}
