//! Untagging an account.

use diverge_sdk::daemon::endpoints::accounts::untag::client::request;
use diverge_sdk::daemon::endpoints::accounts::untag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, accounts, tags};

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
/// for an account the grants do not reach, or a tag the grant does
/// not cover; else the tags off the account, a tag not held nothing
/// to take, and an empty list changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::accounts::holds_tagging(&standing, Tagging::Untag) {
        return Ok(Frame::Forbidden);
    }
    let Some(account) = accounts::by_reference(&mut tx, &frame.account, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_connected(account.id).await;
    if !judge::accounts::tagging(&standing, Tagging::Untag, &account, connected, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    accounts::set_tags(&mut tx, account.id, &tags::without(&account.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Accounts);
    Ok(Frame::Untagged)
}
