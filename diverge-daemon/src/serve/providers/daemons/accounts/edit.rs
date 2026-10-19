//! Replacing an account's mode on a daemon record, by a patch.

use diverge_sdk::daemon::endpoints::providers::daemons::accounts::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::providers::daemons::accounts::edit::server::response::Frame;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::inner;
use crate::serve::reply;
use crate::store::{self, providers_daemons};

/// Answer the edit and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `edit` grant over daemon records at all;
/// `NotFound` for no such record or no such account on it;
/// `Forbidden` for a record the grants do not reach; the `patch`
/// error for a patch that could not be applied, or whose result gives
/// no mode — the document the patch is applied to has none, since the
/// daemon answers a credential to nobody; else the mode replaced, and
/// the connection held as the account, if any, let go, so that the
/// next one opened presents the new credential.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_daemons::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = providers_daemons::by_name(&mut tx, &frame.daemon, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_daemon_connected(&record.name).await;
    if !judge::providers_daemons::over(&standing, Over::Edit, &record, connected) {
        return Ok(Frame::Forbidden);
    }
    let result = match inner::patched(&Editable { mode: None }, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    let Some(mode) = result.mode else {
        return Ok(Frame::Error(reply::patch("the mode is not given")));
    };
    if !providers_daemons::accounts::set_mode(&mut tx, record.id, &frame.name, &mode).await? {
        return Ok(Frame::NotFound);
    }
    tx.commit().await?;
    daemon.live.remove_account(&record.name, &frame.name).await;
    daemon.live.changed(Kind::ProvidersDaemons);
    Ok(Frame::Edited)
}
