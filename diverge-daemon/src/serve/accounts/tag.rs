//! Tagging an account.

use diverge_sdk::daemon::endpoints::accounts::tag::client::request;
use diverge_sdk::daemon::endpoints::accounts::tag::server::response::Frame;
use diverge_sdk::daemon::grant::Tagging;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, accounts, tags};

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
/// for an account the grants do not reach, or a tag the grant does
/// not cover; else the tags on the account, a tag held already held
/// still, and an empty list changing nothing.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::accounts::holds_tagging(&standing, Tagging::Tag) {
        return Ok(Frame::Forbidden);
    }
    let Some(account) = accounts::by_reference(&mut tx, &frame.account, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_connected(account.id).await;
    if !judge::accounts::tagging(&standing, Tagging::Tag, &account, connected, &frame.tags) {
        return Ok(Frame::Forbidden);
    }
    accounts::set_tags(&mut tx, account.id, &tags::with(&account.tags, &frame.tags)).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Accounts);
    Ok(Frame::Tagged)
}
