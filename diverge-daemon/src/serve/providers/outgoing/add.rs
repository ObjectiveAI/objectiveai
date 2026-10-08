//! Adding an outgoing provider.

use std::sync::Arc;

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::providers::outgoing::add::client::request;
use diverge_sdk::daemon::endpoints::providers::outgoing::add::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::providers;
use crate::serve::reply;
use crate::store::{self, providers_outgoing};

/// Answer the add and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `add` grant; `Exists` for an address
/// another provider has; else the provider on record, and dialled
/// from now on.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_outgoing::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    let new = providers_outgoing::New {
        address: frame.address.clone(),
        mode: frame.mode,
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    match providers_outgoing::create(&mut tx, &new).await? {
        providers_outgoing::Created::Created(_) => {}
        providers_outgoing::Created::Exists => return Ok(Frame::Exists),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersOutgoing);
    providers::start(daemon, frame.address).await;
    Ok(Frame::Added)
}
