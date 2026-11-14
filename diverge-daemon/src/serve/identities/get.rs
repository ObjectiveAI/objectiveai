//! Getting one of the host's identities, with its tags.

use diverge_sdk::daemon::endpoints::identities::get::client::request;
use diverge_sdk::daemon::endpoints::identities::get::server::response::Frame;
use diverge_sdk::daemon::grant::identities::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::held;
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

/// `Forbidden` with no `get` grant at all; `Error` for files that could
/// not be read; `NotFound` for an identity not on the host; `Forbidden`
/// for an identity the grants do not reach; else the identity as a list
/// reports it, with its tags.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::identities::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let found = match held(&mut conn, daemon).await? {
        Ok(identities) => identities
            .into_iter()
            .find(|identity| identity.issuer == frame.identity.issuer && identity.subject == frame.identity.subject),
        Err(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    let Some(identity) = found else {
        return Ok(Frame::NotFound);
    };
    if !judge::identities::over(&standing, Over::Get, &identity) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(identity))
}
