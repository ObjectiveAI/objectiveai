//! Tagged: the tags on a agent template.

use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::daemon::endpoints::agents::templates::tag::client::request;
use diverge_sdk::daemon::endpoints::agents::templates::tag::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents_templates, tags};
use super::in_use;

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
/// for a template the grants do not reach, or a tag the grant does not
/// cover; else the tags on the template, and an empty list
/// changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::agents_templates::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = agents_templates::by_id(&mut tx, &frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let held = store::in_use::agents_templates(&mut tx).await?;
    if !judge::agents_templates::tagging(&standing, Tagging::Tag, &record, in_use(&held, &record), &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    agents_templates::set_tags(&mut tx, &record.id, &tags::with(&record.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::AgentsTemplates);
    Ok(Frame::Tagged)
}
