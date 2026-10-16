//! Setting a vault key: made, or its value replaced.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::vault::set::client::request;
use diverge_sdk::daemon::endpoints::vault::set::server::response::Frame;
use diverge_sdk::daemon::grant::vault::{Make, Over};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, vault};

/// Answer the set and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// For a key the vault holds: `Forbidden` with no `set` grant over
/// it; for a key it does not: `Forbidden` without the `set` make;
/// else the key holding the value — made, or replaced, the records
/// binding it made with the new value at their next start.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    let allowed = match vault::by_key(&mut tx, &frame.key, true).await? {
        Some(entry) => judge::vault::holds(&standing, Over::Set) && judge::vault::over(&standing, Over::Set, &entry),
        None => judge::vault::make(&standing, Make::Set),
    };
    if !allowed {
        return Ok(Frame::Forbidden);
    }
    let creator = Creator::Client(Client {
        identity: standing.identity.clone(),
    });
    vault::set(&mut tx, &frame.key, &frame.value, &creator).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Vault);
    Ok(Frame::Set)
}
