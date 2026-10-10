//! Exposing a tool to another daemon: its container up and held, and
//! what joins it answered.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::tools::expose::client::request;
use diverge_sdk::daemon::endpoints::tools::expose::server::response::{Exposed, Frame};
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Found, READ_ONLY, reaches, resolve};
use crate::containers::{self, User};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, key};
use crate::serve::{files, reply};
use crate::store;

/// Send the exposure, hold the scope until the tool's run ends or the
/// client cancels, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// The one sentence a connected tool's expose answers with: it is
/// another daemon's, and that daemon exposes it.
const CONNECTED: &str = "a connected tool is another daemon's, and is exposed there";

/// `Forbidden` with no `expose` grant at all; `NotFound`; `Forbidden`
/// for a tool the grants do not reach; the error for a dependency,
/// which is its agent's, and for a connected tool, which is another
/// daemon's; the container started, or touched if it runs, as a user
/// of the exposure's own, which failing is the `Error`; else exactly
/// one `Exposed` — the provider the container runs on, what this
/// daemon is known as there, the container's id, and a key minted for
/// this exposure alone, kept by its hash in memory and spent by the
/// one connection that presents it — and then the scope held until
/// the run ends or the client cancels, on either of which the key
/// admits nothing more and the exposure lets the tool go.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::tools::holds(&standing, Over::Expose) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(found) = resolve(&mut conn, daemon, &frame.tool, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let reached = reaches(&mut conn, daemon, &standing, Over::Expose, &found).await?;
    drop(conn);
    if !reached {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Found::Record(tool) = found else {
        reply::reply(scope, &Frame::Error(reply::failure(&READ_ONLY))).await;
        return Ok(());
    };
    if tool.is_connected() {
        reply::reply(scope, &Frame::Error(reply::failure(&CONNECTED))).await;
        return Ok(());
    }
    let user = User::Exposure(daemon.live.mint_exposure().await);
    let run = match containers::use_tool(daemon, &tool, user).await {
        Ok(run) => run,
        Err(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    };
    let Some(id) = run.container.clone() else {
        containers::release(daemon, run.id, user).await;
        reply::reply(scope, &Frame::Error(reply::failure(&"the tool's run has no container of this daemon's"))).await;
        return Ok(());
    };
    let authorization = key::mint();
    let hash = key::hash(&authorization);
    daemon.live.expose(hash.clone(), tool.id).await;
    let exposed = Exposed {
        provider: run.provider.clone(),
        identity: daemon.live.known_as(&run.provider).await,
        id,
        authorization,
    };
    reply::reply(scope, &Frame::Exposed(exposed)).await;
    let mut ended = run.ended.subscribe();
    tokio::select! {
        () = async {
            while !*ended.borrow_and_update() {
                if ended.changed().await.is_err() {
                    break;
                }
            }
        } => {}
        () = files::cancelled(scope) => {}
    }
    daemon.live.unexpose(&hash).await;
    containers::release(daemon, run.id, user).await;
    Ok(())
}
