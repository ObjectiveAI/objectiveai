//! Getting one incoming credential.

use diverge_sdk::daemon::endpoints::providers::incoming::get::client::request;
use diverge_sdk::daemon::endpoints::providers::incoming::get::server::response::Frame;
use diverge_sdk::daemon::grant::providers_incoming::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_incoming};

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
/// for a credential the grants do not reach; else the credential as
/// a list reports it, never its key.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_incoming::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(credential) = providers_incoming::by_identity(&mut conn, &frame.identity, false).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_provider_connected(&credential.provider()).await;
    if !judge::providers_incoming::over(&standing, Over::Get, &credential, connected) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(credential.report(connected)))
}
