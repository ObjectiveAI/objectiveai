//! Tagging an agent.

use diverge_sdk::daemon::endpoints::agents::tag::client::request;
use diverge_sdk::daemon::endpoints::agents::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::active;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents, tags};

/// Answer the tag and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `tag` grant at all; `NotFound`; `Forbidden`
/// for an agent the grants do not reach, or a tag the grant does not
/// cover; else the tags on the agent, a tag held already held still.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::agents::tagging(&standing, Tagging::Tag, &agent, active(daemon, agent.id).await, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    agents::set_tags(&mut tx, agent.id, &tags::with(&agent.tags, &frame.tags)).await?;
    tx.commit().await?;
    Ok(Frame::Tagged)
}
