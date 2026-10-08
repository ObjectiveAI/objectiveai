//! Untagging an agent.

use diverge_sdk::daemon::endpoints::agents::untag::client::request;
use diverge_sdk::daemon::endpoints::agents::untag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::active;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents, tags};

/// Answer the untag and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `untag` grant at all; `NotFound`; `Forbidden`
/// for an agent the grants do not reach, or a tag the grant does not
/// cover; else the tags off the agent, a tag not held not held still.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents::holds_tagging(&standing, Tagging::Untag) {
        return Ok(Frame::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut tx, &frame.agent, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::agents::tagging(&standing, Tagging::Untag, &agent, active(daemon, agent.id).await, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    agents::set_tags(&mut tx, agent.id, &tags::without(&agent.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Agents);
    Ok(Frame::Untagged)
}
