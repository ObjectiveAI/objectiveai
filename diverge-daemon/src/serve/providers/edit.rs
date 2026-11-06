//! Editing a provider's record.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::providers::edit::client::request;
use diverge_sdk::daemon::endpoints::providers::edit::server::response::Frame;
use diverge_sdk::daemon::grant::providers::Action;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::daemon::lifecycle::Reason;

use crate::containers::lifecycle::End;
use crate::providers;

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

/// `Forbidden` with no `edit` grant; `NotFound`; else every member given
/// written over the record's, the book read again, and the lists told.
/// A change to the direction or the discover flag closes the
/// connection, its runs ending as `provider_edited`, and the provider is
/// reached again as the record now stands.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers::holds(&standing, Action::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(kept) = store::providers::by_identity(&mut tx, &frame.identity, true).await? else {
        return Ok(Frame::NotFound);
    };
    let mut record = kept.clone();
    if let Some(direction) = frame.direction {
        record.direction = direction;
    }
    if let Some(discover) = frame.discover {
        record.discover = discover;
    }
    if let Some(description) = frame.description {
        record.description = description;
    }
    store::providers::edit(&mut tx, &record).await?;
    tx.commit().await?;
    daemon.providers.reload(&daemon.store).await?;
    if record.direction != kept.direction || record.discover != kept.discover {
        providers::reconcile(daemon, &record.identity, Some(End::Unexpected(Reason::ProviderEdited))).await;
    }
    daemon.live.changed(Kind::Providers);
    Ok(Frame::Edited)
}
