//! Which database the daemon serves.

use diverge_sdk::daemon::endpoints::postgres::get::client::request;
use diverge_sdk::daemon::endpoints::postgres::get::server::response::Frame;
use diverge_sdk::daemon::grant::postgres::Action;
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

/// `Forbidden` without the `get` grant; else the mode — local, or the
/// remote URL with its password taken out.
async fn serve(_: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::postgres::holds(&standing, Action::Get) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Mode(daemon.database.reported()))
}
