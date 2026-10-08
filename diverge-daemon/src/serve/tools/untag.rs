//! Untagging a tool.

use diverge_sdk::daemon::endpoints::tools::untag::client::request;
use diverge_sdk::daemon::endpoints::tools::untag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tags, tools};

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
/// for a tool the grants do not reach, or a tag the grant does not
/// cover; else the tags off the tool, a tag not held not held still.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds_tagging(&standing, Tagging::Untag) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    let attached = agents_of(&mut tx, tool.id).await?;
    if !judge::tools::tagging(&standing, Tagging::Untag, &tool, active(daemon, tool.id).await, &attached, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    tools::set_tags(&mut tx, tool.id, &tags::without(&tool.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Tools);
    Ok(Frame::Untagged)
}
