//! Taking a route up.

use diverge_sdk::daemon::endpoints::tools::routes::delete::client::request;
use diverge_sdk::daemon::endpoints::tools::routes::delete::server::response::Frame;
use diverge_sdk::daemon::grant::routes::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::in_use;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, routes, tools};

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
/// for a route the grants do not reach; `InUse` while an active
/// container is served through it; else the route up: the position is
/// answered by the deployer again.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::routes::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(route) = routes::by_path(&mut tx, &frame.path, true).await? else {
        return Ok(Frame::NotFound);
    };
    let tool = tools::by_id(&mut tx, route.tool, false).await?;
    let tool_name = tool.as_ref().and_then(|tool| tool.name.as_deref());
    if !judge::routes::over(&standing, Over::Delete, &route, tool_name) {
        return Ok(Frame::Forbidden);
    }
    if in_use(daemon, &route).await {
        return Ok(Frame::InUse);
    }
    routes::delete(&mut tx, &frame.path).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Routes);
    daemon.live.changed(Kind::Tools);
    Ok(Frame::Deleted)
}
