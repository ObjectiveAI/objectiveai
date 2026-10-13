//! Tagged: the tags on a dependency template.

use diverge_sdk::daemon::endpoints::dependencies::tag::client::request;
use diverge_sdk::daemon::endpoints::dependencies::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{agents_of, runs_of};
use crate::daemon::{Daemon, Kind};
use crate::judge::filter::dependencies::Facts;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, dependencies, tags};

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
    if !judge::dependencies::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = dependencies::by_id(&mut tx, &frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let agents = agents_of(&mut tx, &record.id).await?;
    let runs = runs_of(daemon, &record.id).await;
    let facts = Facts {
        record: &record,
        agents: &agents,
        runs: &runs,
    };
    if !judge::dependencies::tagging(&standing, Tagging::Tag, &facts, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    dependencies::set_tags(&mut tx, &record.id, &tags::with(&record.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Dependencies);
    Ok(Frame::Tagged)
}
