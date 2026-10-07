//! Editing an account.

use std::net::IpAddr;

use diverge_sdk::daemon::edit::Change;
use diverge_sdk::daemon::endpoints::accounts::edit::client::request;
use diverge_sdk::daemon::endpoints::accounts::edit::server::response::Frame;
use diverge_sdk::daemon::grant::accounts::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{NamedRoles, named_roles};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, key};
use crate::serve::reply;
use crate::store::{self, accounts};

/// Answer the edit and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `edit` grant at all; `NotFound`; `Forbidden`
/// for an account the grants do not reach, or a role named the caller
/// holds no `grant` over; `NoRole`; `Alone` for a change that would
/// leave the account with neither a name nor a credential, or take
/// the credential from one a client is connected as, or the name from
/// one a container runs under; `InUse` for a name or an identity
/// another account has; else the account as the request states, whole
/// or not at all, with a new key when a credential was set, answered
/// here and never again — and the connection a client holds as the
/// account is not dropped by it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::accounts::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(account) = accounts::by_reference(&mut tx, &frame.account, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_connected(account.id).await;
    if !judge::accounts::over(&standing, Over::Edit, &account, connected) {
        return Ok(Frame::Forbidden);
    }
    let roles = match &frame.roles {
        None => None,
        Some(Change::Delete) => Some(Vec::new()),
        Some(Change::Set(names)) => match named_roles(&mut tx, &standing, names).await? {
            NamedRoles::Roles(roles) => Some(roles),
            NamedRoles::NoRole => return Ok(Frame::NoRole),
            NamedRoles::Forbidden => return Ok(Frame::Forbidden),
        },
    };
    let name = match frame.name {
        None => account.name.clone(),
        Some(Change::Delete) => None,
        Some(Change::Set(name)) => Some(name),
    };
    let mut minted = None;
    let (identity, address, key_hash): (Option<String>, Option<IpAddr>, Option<String>) = match frame.credential {
        None => (account.identity.clone(), account.address, account.key_hash.clone()),
        Some(Change::Delete) => (None, None, None),
        Some(Change::Set(credential)) => {
            let key = key::mint();
            let hash = key::hash(&key);
            minted = Some(key);
            (Some(credential.identity), credential.address, Some(hash))
        }
    };
    let description = match frame.description {
        None => account.description.clone(),
        Some(Change::Delete) => None,
        Some(Change::Set(description)) => Some(description),
    };
    let credential_taken = account.identity.is_some() && identity.is_none();
    let name_taken = account.name.is_some() && name.is_none();
    if (name.is_none() && identity.is_none()) || (credential_taken && connected) || (name_taken && store::in_use::account(&mut tx, account.id).await?) {
        return Ok(Frame::Alone);
    }
    let columns = accounts::Columns {
        name,
        identity,
        address,
        key_hash,
        description,
    };
    if let accounts::Updated::InUse = accounts::update(&mut tx, account.id, &columns).await? {
        return Ok(Frame::InUse);
    }
    if let Some(roles) = roles {
        accounts::set_roles(&mut tx, account.id, &roles).await?;
    }
    tx.commit().await?;
    Ok(Frame::Edited(minted))
}

