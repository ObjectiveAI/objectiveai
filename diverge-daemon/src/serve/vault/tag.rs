//! Tagging a vault key.

use diverge_sdk::daemon::endpoints::vault::tag::client::request;
use diverge_sdk::daemon::endpoints::vault::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tags, vault};

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
/// for a key the grants do not reach, or a tag the grant does not
/// cover; else the tags on the key.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::vault::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(entry) = vault::by_key(&mut tx, &frame.key, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::vault::tagging(&standing, Tagging::Tag, &entry, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    vault::set_tags(&mut tx, &frame.key, &tags::with(&entry.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Vault);
    Ok(Frame::Tagged)
}
