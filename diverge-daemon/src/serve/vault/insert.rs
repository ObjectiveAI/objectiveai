//! Inserting a value into a vault key: the key made with it as its
//! first value, the value appended, or one slot's value replaced.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::vault::insert::client::request;
use diverge_sdk::daemon::endpoints::vault::insert::server::response::Frame;
use diverge_sdk::daemon::grant::vault::{Make, Over};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, vault};

/// Answer the insert and finish the scope.
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
/// 1. For a key the vault holds: `Forbidden` with no `insert` grant over
///    it; else the value appended as a new slot where none is named,
///    or the slot named replaced — `Inserted`; `NotFound` for a slot the
///    key does not have; `Locked` for one a running container holds,
///    left as it is.
/// 2. For a key it does not hold, a slot named: `NotFound` to a
///    standing that holds `insert` over any key or the `insert` make, and
///    `Forbidden` to one that holds neither, which is told nothing of
///    what the vault has.
/// 3. For a key it does not hold, no slot named: `Forbidden` without
///    the `insert` make; else the key made holding the value as its slot
///    `0` — `Inserted`.
///
/// A key some record binds is inserted into like any other: the next activation
/// of the record's container may be handed the new value.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    let allowed = match vault::by_key(&mut tx, &frame.key, true).await? {
        Some(entry) => judge::vault::holds(&standing, Over::Insert) && judge::vault::over(&standing, Over::Insert, &entry),
        None if frame.slot.is_some() => {
            return Ok(if judge::vault::holds(&standing, Over::Insert) || judge::vault::make(&standing, Make::Insert) {
                Frame::NotFound
            } else {
                Frame::Forbidden
            });
        }
        None => judge::vault::make(&standing, Make::Insert),
    };
    if !allowed {
        return Ok(Frame::Forbidden);
    }
    let creator = Creator::Client(Client {
        identity: standing.identity.clone(),
    });
    match vault::insert(&mut tx, &frame.key, frame.slot, &frame.value, &creator).await? {
        vault::Insert::Created | vault::Insert::Appended(_) | vault::Insert::Replaced => {}
        vault::Insert::NotFound => return Ok(Frame::NotFound),
        vault::Insert::Locked => return Ok(Frame::Locked),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Vault);
    Ok(Frame::Inserted)
}
