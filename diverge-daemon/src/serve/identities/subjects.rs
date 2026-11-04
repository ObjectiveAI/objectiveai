//! Listing the subjects signed in at one issuer.

use diverge_sdk::daemon::endpoints::identities::subjects::client::request;
use diverge_sdk::daemon::endpoints::identities::subjects::server::response::Frame;
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

/// `Forbidden` with no `subjects` grant; else every subject an identity
/// is signed in as at the issuer named, in order — none where no
/// identity is signed in there; the files that could not be read the
/// `Error`.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    drop(conn);
    if !judge::identities::holds(&standing, Action::Subjects) {
        return Ok(Frame::Forbidden);
    }
    Ok(match daemon.authenticator.identities().list().await {
        Ok(names) => Frame::Listed(names.into_iter().filter(|name| name.issuer == frame.issuer).map(|name| name.subject).collect()),
        Err(error) => Frame::Error(reply::failure(&error)),
    })
}
