//! Tagging a daemon record.

use diverge_sdk::daemon::endpoints::providers::daemons::tag::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_daemons, tags};

/// Answer the tag and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `tag` grant at all; `NotFound`; `Forbidden` for
/// a record the grants do not reach, or a tag the grant does not cover;
/// else the tags on the record, a tag held already held still, and an
/// empty list changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_daemons::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = providers_daemons::by_name(&mut tx, &frame.name, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_daemon_connected(&record.name).await;
    if !judge::providers_daemons::tagging(&standing, Tagging::Tag, &record, connected, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    providers_daemons::set_tags(&mut tx, record.id, &tags::with(&record.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersDaemons);
    Ok(Frame::Tagged)
}
