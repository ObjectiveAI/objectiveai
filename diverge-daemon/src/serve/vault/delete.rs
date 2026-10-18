//! Deleting a vault key, or one of its values.

use diverge_sdk::daemon::endpoints::vault::delete::client::request;
use diverge_sdk::daemon::endpoints::vault::delete::server::response::Frame;
use diverge_sdk::daemon::grant::vault::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, vault};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The rules, in order:
///
/// 1. `Forbidden` with no `delete` grant at all.
/// 2. `NotFound` for a key the vault does not hold — a slot named on
///    such a key included.
/// 3. `Forbidden` for a key the grants do not reach.
/// 4. For the whole key — no slot named: `InUse` while an agent or a
///    tool binds it; `Locked` while a running container holds any
///    value of it; else the key gone with every value — `Deleted`.
/// 5. For a slot: `NotFound` for a number the key has no slot by;
///    `Locked` while a running container holds it; `Last` for the
///    key's only slot, which the key's own delete is for; else the
///    slot gone — `Deleted`.
///
/// Nothing is changed by any answer but `Deleted`.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::vault::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(entry) = vault::by_key(&mut tx, &frame.key, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::vault::over(&standing, Over::Delete, &entry) {
        return Ok(Frame::Forbidden);
    }
    match vault::delete(&mut tx, &frame.key, frame.slot).await? {
        vault::Deleted::Deleted => {}
        vault::Deleted::NotFound => return Ok(Frame::NotFound),
        vault::Deleted::InUse => return Ok(Frame::InUse),
        vault::Deleted::Locked => return Ok(Frame::Locked),
        vault::Deleted::Last => return Ok(Frame::Last),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Vault);
    Ok(Frame::Deleted)
}
