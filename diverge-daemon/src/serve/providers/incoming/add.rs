//! Adding an incoming credential.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::providers::incoming::add::client::request;
use diverge_sdk::daemon::endpoints::providers::incoming::add::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, key};
use crate::serve::reply;
use crate::store::{self, providers_incoming};

/// Answer the add and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `add` grant; `Exists` for an identity a
/// credential names already; else the credential on record, and its
/// key — minted here, answered here, and never again.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_incoming::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    let minted = key::mint();
    let new = providers_incoming::New {
        identity: frame.credential.identity,
        address: frame.credential.address,
        key_hash: key::hash(&minted),
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    match providers_incoming::create(&mut tx, &new).await? {
        providers_incoming::Created::Created(_) => {}
        providers_incoming::Created::Exists => return Ok(Frame::Exists),
    }
    tx.commit().await?;
    Ok(Frame::Added(minted))
}
