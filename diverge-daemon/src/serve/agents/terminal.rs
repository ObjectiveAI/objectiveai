//! An interactive shell in an agent's container.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::terminal::client::request;
use diverge_sdk::daemon::endpoints::agents::terminal::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::state;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store::{self, agents};

/// Open the terminal, serve it until it ends, and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `exec` grant at all; `NotFound`; `Forbidden` for
/// an agent the grants do not reach; the container started if it was
/// not up, which failing is the `Error`; else the command's stream
/// (see [`files::terminal`]), the container held up while it is open.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents::holds(&standing, Over::Exec) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Exec, &agent, state(daemon, agent.id).await) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    scope.allowed().await;
    let opened = match files::open_agent(daemon, &agent).await {
        Ok(opened) => opened,
        Err(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    };
    opened.touch();
    if let Err(error) = files::terminal::<Frame>(scope, &opened, frame.size).await {
        reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
    }
    Ok(())
}
