//! Untagging a role.

use diverge_sdk::daemon::endpoints::roles::untag::client::request;
use diverge_sdk::daemon::endpoints::roles::untag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, roles, tags};

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
/// for a role the grants do not reach, or a tag the grant does not
/// cover; else the tags off the role.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::roles::holds_tagging(&standing, Tagging::Untag) {
        return Ok(Frame::Forbidden);
    }
    let Some(role) = roles::by_name(&mut tx, &frame.name, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::roles::tagging(&standing, Tagging::Untag, &role, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    roles::set_tags(&mut tx, role.id, &tags::without(&role.tags, &frame.tags)).await?;
    tx.commit().await?;
    Ok(Frame::Untagged)
}
