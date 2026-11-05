//! Adding a remote daemon's record: one account of ours on it.

use diverge_sdk::daemon::endpoints::daemons::add::client::request;
use diverge_sdk::daemon::endpoints::daemons::add::server::response::Frame;
use diverge_sdk::daemon::grant::daemons::Make;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the request and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `add` grant; the `Error` for our identity at
/// another issuer than the remote's, since the handshake between the two
/// speaks one; `Exists` for a name a record has; else the record kept,
/// its tags sorted, and the lists told.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::daemons::make(&standing, Make::Add) {
        return Ok(Frame::Forbidden);
    }
    if frame.identity.issuer != frame.daemon.issuer {
        return Ok(Frame::Error(reply::failure(&"our identity must be at the remote's issuer: the handshake between the two speaks one")));
    }
    let record = store::daemons::Record {
        name: frame.name,
        description: frame.description.unwrap_or_default(),
        daemon: frame.daemon,
        identity: frame.identity,
        tags: store::tags::with(&[], &frame.tags),
        created: chrono::Utc::now(),
        creator: standing.creator.clone(),
    };
    if !store::daemons::create(&mut conn, &record).await? {
        return Ok(Frame::Exists);
    }
    daemon.live.changed(Kind::Daemons);
    Ok(Frame::Added)
}
