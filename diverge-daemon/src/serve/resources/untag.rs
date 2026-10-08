//! Untagged: the tags off a resource.

use diverge_sdk::daemon::endpoints::resources::untag::client::request;
use diverge_sdk::daemon::endpoints::resources::untag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::in_use;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, resources, tags};

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
/// for a resource the grants do not reach, or a tag the grant does not
/// cover; else the tags off the resource, and an empty list
/// changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::resources::holds_tagging(&standing, Tagging::Untag) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = resources::by_id(&mut tx, &frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let held = store::in_use::resources(&mut tx).await?;
    if !judge::resources::tagging(&standing, Tagging::Untag, &record, in_use(&held, &record), &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    resources::set_tags(&mut tx, &record.id, &tags::without(&record.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Resources);
    Ok(Frame::Untagged)
}
