//! An interactive shell in a dependency tool's container.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::dependency_tools::terminal::client::request;
use diverge_sdk::daemon::endpoints::dependency_tools::terminal::server::response::Frame;
use diverge_sdk::daemon::grant::dependency_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::facts::reaches;
use super::resolve::resolve;
use crate::containers::Opened;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store;

/// Open the terminal, serve it until it ends, and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `exec` grant at all; `NotFound` for a tool that
/// is none, or does not run — nothing is started for it; `Forbidden`
/// for a tool the grants do not reach; else the terminal (see
/// [`files::terminal`]), the run held up — with the agent it was deployed
/// for — while it is open.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::dependency_tools::holds(&standing, Over::Exec) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some((tool, Some(run))) = resolve(&mut conn, daemon, &frame.dependency, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let reached = reaches(&mut conn, daemon, &standing, Over::Exec, &tool).await?;
    drop(conn);
    if !reached {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    scope.allowed().await;
    let using = run.file_use();
    let opened = Opened::Tool(run, Some(using));
    opened.touch();
    if let Err(error) = files::terminal::<Frame>(scope, &opened, frame.size).await {
        reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
    }
    Ok(())
}
