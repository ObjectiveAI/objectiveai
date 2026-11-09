//! Untagging a connected tool.

use diverge_sdk::daemon::endpoints::connected_standalone_tools::untag::client::request;
use diverge_sdk::daemon::endpoints::connected_standalone_tools::untag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{load, resolve};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, connected_standalone_tools, tags};

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
    if !judge::connected_standalone_tools::holds_tagging(&standing, Tagging::Untag) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = resolve(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    let loaded = load(&mut tx, daemon, &tool).await?;
    if !judge::connected_standalone_tools::tagging(&standing, Tagging::Untag, &loaded.facts(&tool), &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    connected_standalone_tools::set_tags(&mut tx, tool.id, &tags::without(&tool.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ConnectedStandaloneTools);
    Ok(Frame::Untagged)
}
