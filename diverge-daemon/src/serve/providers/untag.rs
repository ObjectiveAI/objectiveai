//! Untagging a provider's record.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::providers::untag::client::request;
use diverge_sdk::daemon::endpoints::providers::untag::server::response::Frame;
use diverge_sdk::daemon::grant::providers::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the request and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `untag` grant; `NotFound`; else the tags
/// off the record, the book read again, and the lists told.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers::holds(&standing, Action::Untag) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = store::providers::by_identity(&mut tx, &frame.identity, true).await? else {
        return Ok(Frame::NotFound);
    };
    store::providers::set_tags(&mut tx, &record.identity, &store::tags::without(&record.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.providers.reload(&daemon.store).await?;
    daemon.live.changed(Kind::Providers);
    Ok(Frame::Untagged)
}
