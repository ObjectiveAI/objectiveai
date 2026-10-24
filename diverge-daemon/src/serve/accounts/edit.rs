//! Editing an account: a patch to its description and its roles.

use diverge_sdk::daemon::endpoints::accounts::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::accounts::edit::server::response::Frame;
use diverge_sdk::daemon::grant::accounts::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{NamedRoles, named_roles};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner;
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
/// holds no `grant` over; the `patch` error for a patch that could
/// not be applied or a result that is not an account's editable
/// members; `NoRole`; else the account as the result states, whole or
/// not at all. The identity is not editable — an account is its
/// identity — and the connection a client holds as the account is not
/// dropped by an edit.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::accounts::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(account) = accounts::by_identity(&mut tx, &frame.account, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_connected(account.id).await;
    if !judge::accounts::over(&standing, Over::Edit, &account, connected) {
        return Ok(Frame::Forbidden);
    }
    let current = Editable {
        description: account.description.clone(),
        roles: account.roles.clone(),
    };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    let roles = if result.roles != current.roles {
        match named_roles(&mut tx, &standing, &result.roles).await? {
            NamedRoles::Roles(roles) => Some(roles),
            NamedRoles::NoRole => return Ok(Frame::NoRole),
            NamedRoles::Forbidden => return Ok(Frame::Forbidden),
        }
    } else {
        None
    };
    accounts::update(&mut tx, account.id, &accounts::Columns { description: result.description }).await?;
    if let Some(roles) = roles {
        accounts::set_roles(&mut tx, account.id, &roles).await?;
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Accounts);
    daemon.live.changed(Kind::Roles);
    Ok(Frame::Edited)
}
