//! One HTTP request to the frontend of a dependency tool's container.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::dependency_tools::frontend::client::request;
use diverge_sdk::daemon::endpoints::dependency_tools::frontend::server::response::Frame;
use diverge_sdk::shared::mcp::attestation::Caller;
use diverge_sdk::daemon::grant::dependency_tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::facts::reaches;
use super::resolve::resolve;
use crate::containers::Opened;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store;

/// Send the request, serve its answer and its body, and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `frontend` grant at all, or no account; `NotFound` for a tool that
/// is none, or does not run — nothing is started for it; `Forbidden`
/// for a tool the grants do not reach; `NoFrontend` for a tool whose
/// image declares none; else the request (see
/// [`files::frontend`]), the run held up — with the agent it was deployed
/// for — while it is answered.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::dependency_tools::holds(&standing, Over::Frontend) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    // The account behind the request, and the scope's own id: a
    // standing with no account is refused, as a connect refuses one.
    let Some(identity) = standing.identity.clone() else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    let caller = Caller::Client { identity, scope: uuid::Uuid::new_v4().to_string() };
    let Some((tool, Some(run))) = resolve(&mut conn, daemon, &frame.dependency, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let reached = reaches(&mut conn, daemon, &standing, Over::Frontend, &tool).await?;
    drop(conn);
    if !reached {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    scope.allowed().await;
    match files::declares(daemon, &tool.declared.image).await? {
        Ok(true) => {}
        Ok(false) => {
            reply::reply(scope, &Frame::NoFrontend).await;
            return Ok(());
        }
        Err(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    }
    let using = run.file_use();
    let opened = Opened::Tool(run, Some(using));
    opened.touch();
    if let Err(error) = files::frontend::<Frame>(scope, &opened, frame.head, &caller).await {
        reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
    }
    Ok(())
}
