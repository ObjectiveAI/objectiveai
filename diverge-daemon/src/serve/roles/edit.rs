//! Editing a role.

use diverge_sdk::daemon::edit::Change;
use diverge_sdk::daemon::endpoints::roles::edit::client::request;
use diverge_sdk::daemon::endpoints::roles::edit::server::response::Frame;
use diverge_sdk::daemon::grant::roles::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, roles};

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
/// for a role the grants do not reach; else the role as the request
/// states, its description and its grants each replaced whole, taken
/// away, or left — and every account holding it judged by the new
/// grants from its next request on.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::roles::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(role) = roles::by_name(&mut tx, &frame.name, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !judge::roles::over(&standing, Over::Edit, &role) {
        return Ok(Frame::Forbidden);
    }
    let columns = roles::Columns {
        description: match frame.description {
            None => role.description.clone(),
            Some(Change::Delete) => None,
            Some(Change::Set(description)) => Some(description),
        },
        grants: match frame.grants {
            None => role.grants.clone(),
            Some(Change::Delete) => Vec::new(),
            Some(Change::Set(grants)) => grants,
        },
    };
    roles::update(&mut tx, role.id, &columns).await?;
    tx.commit().await?;
    Ok(Frame::Edited)
}
