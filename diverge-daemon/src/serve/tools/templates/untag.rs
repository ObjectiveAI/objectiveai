//! Untagged: the tags off a tool template.

use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::daemon::endpoints::tools::templates::untag::client::request;
use diverge_sdk::daemon::endpoints::tools::templates::untag::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tools_templates, tags};
use super::in_use;

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
/// for a template the grants do not reach, or a tag the grant does not
/// cover; else the tags off the template, and an empty list
/// changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools_templates::holds_tagging(&standing, Tagging::Untag) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = tools_templates::by_id(&mut tx, &frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::tools_templates::tagging(&standing, Tagging::Untag, &record, in_use(&record), &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    tools_templates::set_tags(&mut tx, &record.id, &tags::without(&record.tags, &frame.tags)).await?;
    tx.commit().await?;
    Ok(Frame::Untagged)
}
