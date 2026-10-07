//! The container connections open through the database.

use diverge_sdk::daemon::endpoints::postgres::Connection;
use diverge_sdk::daemon::endpoints::postgres::connections::client::request;
use diverge_sdk::daemon::endpoints::postgres::connections::server::response::Frame;
use diverge_sdk::daemon::grant::postgres::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::database;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` without the `connections` grant; else every
/// connection a container holds open now, oldest opened first, one
/// frame each, the container named as the database names it — and
/// nothing at all, which is an answer, when none is open. One whose
/// container's record is gone meanwhile is not listed.
async fn serve(scope: &ScopeHandle, _: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::postgres::holds(&standing, Action::Connections) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    for (key, opened) in daemon.live.connections().await {
        if let Some(container) = database::container_of(&mut conn, key).await? {
            reply::reply(scope, &Frame::Connection(Connection { container, opened })).await;
        }
    }
    Ok(())
}
