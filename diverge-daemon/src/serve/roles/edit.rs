//! Editing a role: a patch to its description and its grants.

use diverge_sdk::daemon::endpoints::roles::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::roles::edit::server::response::Frame;
use diverge_sdk::daemon::grant::roles::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner;
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
/// for a role the grants do not reach; the `patch` error for a patch
/// that could not be applied or a result that is not a role's
/// editable members; else the role as the result states — and every
/// account holding it judged by the new grants from its next request
/// on.
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
    let current = Editable {
        description: role.description.clone(),
        grants: role.grants.clone(),
    };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    let columns = roles::Columns {
        description: result.description,
        grants: result.grants,
    };
    roles::update(&mut tx, role.id, &columns).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::Roles);
    Ok(Frame::Edited)
}
