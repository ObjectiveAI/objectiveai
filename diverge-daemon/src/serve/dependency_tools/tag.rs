//! Tagged: the tags on a dependency tool.

use diverge_sdk::daemon::endpoints::dependency_tools::tag::client::request;
use diverge_sdk::daemon::endpoints::dependency_tools::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{load, resolve};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, dependency_tools, tags};

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
/// for a tool the grants do not reach, or a tag the grant does not
/// cover; else the tags on the tool, and an empty list changing
/// nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::dependency_tools::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some((tool, _)) = resolve(&mut tx, daemon, &frame.dependency, true).await? else {
        return Ok(Frame::NotFound);
    };
    let Some(loaded) = load(&mut tx, daemon, &tool).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::dependency_tools::tagging(&standing, Tagging::Tag, &loaded.facts(&tool), &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    dependency_tools::set_tags(&mut tx, tool.id, &tags::with(&tool.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::DependencyTools);
    Ok(Frame::Tagged)
}
