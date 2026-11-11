//! One HTTP request to the frontend of an agent's container.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::frontend::client::request;
use diverge_sdk::daemon::endpoints::agents::frontend::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::shared::mcp::attestation::Caller;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::state;
use crate::containers;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store::{self, agents, agents_templates};

/// Send the request, serve its answer and its body, and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `frontend` grant at all, or no account;
/// `NotFound`; `Forbidden` for an agent the grants do not reach;
/// `NoFrontend` for an agent whose image declares none, before anything
/// is started; the container started if it was not up, which failing
/// is the `Error`; else the request (see [`files::frontend`]), the
/// container held up while it is answered.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents::holds(&standing, Over::Frontend) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    // The account behind the request, and the scope's own id: a
    // standing with no account is refused, as a tool's frontend
    // refuses one.
    let Some(identity) = standing.identity.clone() else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    let caller = Caller::Client { identity, scope: uuid::Uuid::new_v4().to_string() };
    let Some(agent) = agents::by_reference(&mut conn, &frame.agent, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    let image = agents_templates::by_id(&mut conn, &agent.template, false).await?.map(|record| containers::image(&record.template.image));
    drop(conn);
    if !judge::agents::over(&standing, Over::Frontend, &agent, state(daemon, agent.id).await) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    scope.allowed().await;
    let Some(image) = image else {
        reply::reply(scope, &Frame::Error(reply::failure(&format!("the agent's template `{}` is gone", agent.template)))).await;
        return Ok(());
    };
    match files::declares(daemon, &image).await? {
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
    let opened = match files::open_agent(daemon, &agent).await {
        Ok(opened) => opened,
        Err(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    };
    opened.touch();
    if let Err(error) = files::frontend::<Frame>(scope, &opened, frame.head, &caller).await {
        reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
    }
    Ok(())
}
