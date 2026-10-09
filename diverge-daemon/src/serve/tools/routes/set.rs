//! Putting a route down.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::tools::routes::set::client::request;
use diverge_sdk::daemon::endpoints::tools::routes::set::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, routes, tools};

/// Answer the set and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `set` grant; `NoTool`; `Mismatch` for a
/// tool not made from the position's template — a connected tool,
/// made from none, among them; `Exists` for a position with a
/// route; else the route down: a run reaching the position is served
/// the tool and asks no deployer.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::routes::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut tx, &frame.tool, false).await? else {
        return Ok(Frame::NoTool);
    };
    if tool.template() != Some(frame.path.template.as_str()) {
        return Ok(Frame::Mismatch);
    }
    let new = routes::New {
        path: frame.path,
        tool: tool.id,
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    match routes::create(&mut tx, &new).await? {
        routes::Created::Created => {}
        routes::Created::Exists => return Ok(Frame::Exists),
    }
    tx.commit().await?;
    daemon.live.answered.notify_waiters();
    daemon.live.changed(Kind::Routes);
    daemon.live.changed(Kind::Tools);
    Ok(Frame::Set)
}
