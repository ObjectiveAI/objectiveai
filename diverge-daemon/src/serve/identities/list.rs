//! Listing the host's identities, each with its tags.

use diverge_sdk::daemon::endpoints::identities::list::client::request;
use diverge_sdk::daemon::endpoints::identities::list::server::response::Frame;
use diverge_sdk::daemon::grant::identities::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::held;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store;

/// Answer the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `list` grant at all; else every identity on the
/// host that the request's filter lets through and a grant of `list`
/// reaches, each with its tags, by issuer and then subject; the files
/// that could not be read the `Error`.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::identities::holds(&standing, Over::List) {
        return Ok(Frame::Forbidden);
    }
    let identities = match held(&mut conn, daemon).await? {
        Ok(identities) => identities,
        Err(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    Ok(Frame::Listed(
        identities
            .into_iter()
            .filter(|identity| filter::identities::test(&frame.filter, identity) && judge::identities::over(&standing, Over::List, identity))
            .collect(),
    ))
}
