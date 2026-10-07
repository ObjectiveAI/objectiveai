//! Creating an account.

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::endpoints::accounts::Definition;
use diverge_sdk::daemon::endpoints::accounts::create::client::request;
use diverge_sdk::daemon::endpoints::accounts::create::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{NamedRoles, named_roles};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, key};
use crate::serve::reply;
use crate::store::{self, accounts};

/// Answer the create and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `create` grant or the `grant` grant over
/// a role named; `NoRole` for a role the daemon does not have;
/// `Exists` for a name or an identity another account has; else the
/// account made, holding its roles from the first, with the key
/// minted when a credential was given, answered here and never
/// again.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::accounts::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    let roles = match named_roles(&mut tx, &standing, &frame.roles).await? {
        NamedRoles::Roles(roles) => roles,
        NamedRoles::NoRole => return Ok(Frame::NoRole),
        NamedRoles::Forbidden => return Ok(Frame::Forbidden),
    };
    let (name, credential) = match frame.definition {
        Definition::Named { name, credential } => (Some(name), credential),
        Definition::Unnamed { credential } => (None, Some(credential)),
    };
    let minted = credential.as_ref().map(|_| key::mint());
    let new = accounts::New {
        name,
        identity: credential.as_ref().map(|credential| credential.identity.clone()),
        address: credential.and_then(|credential| credential.address),
        key_hash: minted.as_deref().map(key::hash),
        description: frame.description,
        creator: Creator::Client(Client {
            identity: standing.identity.clone(),
        }),
    };
    let id = match accounts::create(&mut tx, &new).await? {
        accounts::Created::Created(id) => id,
        accounts::Created::Exists => return Ok(Frame::Exists),
    };
    accounts::set_roles(&mut tx, id, &roles).await?;
    tx.commit().await?;
    Ok(Frame::Created(minted))
}
