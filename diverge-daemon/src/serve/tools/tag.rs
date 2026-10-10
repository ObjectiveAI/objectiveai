//! Tagging a tool.

use diverge_sdk::daemon::endpoints::tools::tag::client::request;
use diverge_sdk::daemon::endpoints::tools::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Found, READ_ONLY, active, agents_of, resolve};
use crate::daemon::{Daemon, Kind};
use crate::judge::filter::tools::Facts;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tags, tools};

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
/// cover; the error for a dependency, which carries no tags; else the tags on the tool, a tag held already held still.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(found) = resolve(&mut tx, daemon, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    let tool = match found {
        Found::Record(tool) => {
            let attached = agents_of(&mut tx, tool.id).await?;
            if !judge::tools::tagging(&standing, Tagging::Tag, &Facts::record(&tool, active(daemon, tool.id).await, &attached), &frame.tags) {
                return Ok(Frame::Forbidden);
            }
            tool
        }
        Found::Dependency(run) => {
            let reached = Facts::dependency(&run).is_some_and(|facts| judge::tools::tagging(&standing, Tagging::Tag, &facts, &frame.tags));
            return Ok(if reached { Frame::Error(reply::failure(&READ_ONLY)) } else { Frame::Forbidden });
        }
    };
    tools::set_tags(&mut tx, tool.id, &tags::with(&tool.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Tools);
    Ok(Frame::Tagged)
}
