//! Adding a provider's record.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::providers::add::client::request;
use diverge_sdk::daemon::endpoints::providers::add::server::response::Frame;
use diverge_sdk::daemon::grant::providers::Make;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
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

/// `Forbidden` with no `add` grant; `Exists` for an identity a record
/// has; else the record kept — `accept_daemons` absent kept as `true` —
/// its tags sorted, the book read again, an enabled outgoing one
/// dialled, and the lists told. No connection is the provider's yet:
/// what the record says is held open on it is opened when it connects,
/// by [`sync`](providers::sync).
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers::make(&standing, Make::Add) {
        return Ok(Frame::Forbidden);
    }
    let record = store::providers::Record {
        identity: frame.identity,
        direction: frame.direction,
        announcement: frame.announcement,
        accept_daemons: frame.accept_daemons.unwrap_or(true),
        messageable: frame.messageable,
        enabled: frame.enabled.unwrap_or(true),
        description: frame.description.unwrap_or_default(),
        tags: store::tags::with(&[], &frame.tags),
        created: chrono::Utc::now(),
        creator: standing.creator.clone(),
    };
    if !store::providers::create(&mut conn, &record).await? {
        return Ok(Frame::Exists);
    }
    drop(conn);
    daemon.providers.reload(&daemon.store).await?;
    providers::reconcile(daemon, &record.identity, None).await;
    daemon.live.changed(Kind::Providers);
    Ok(Frame::Added)
}
