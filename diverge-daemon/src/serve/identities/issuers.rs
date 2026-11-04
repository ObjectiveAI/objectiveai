//! Listing the issuers the host's identities are signed in at.

use diverge_sdk::daemon::endpoints::identities::issuers::client::request;
use diverge_sdk::daemon::endpoints::identities::issuers::server::response::Frame;
use diverge_sdk::daemon::grant::identities::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
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

/// `Forbidden` with no `issuers` grant; else every issuer at least one
/// identity is signed in at, each once, in order; the files that could
/// not be read the `Error`.
async fn serve(_frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    drop(conn);
    if !judge::identities::holds(&standing, Action::Issuers) {
        return Ok(Frame::Forbidden);
    }
    Ok(match daemon.authenticator.identities().list().await {
        Ok(names) => {
            let mut issuers: Vec<String> = names.into_iter().map(|name| name.issuer).collect();
            issuers.dedup();
            Frame::Listed(issuers)
        }
        Err(error) => Frame::Error(reply::failure(&error)),
    })
}
