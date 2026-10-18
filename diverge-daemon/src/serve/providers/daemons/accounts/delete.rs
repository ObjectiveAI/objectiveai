//! Forgetting an account on a daemon record.

use diverge_sdk::daemon::endpoints::providers::daemons::accounts::delete::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::accounts::delete::server::response::Frame;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, in_use, providers_daemons};

/// Answer the delete and finish the scope.
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
/// `Forbidden` for a record the grants do not reach; `InUse` while a
/// connected tool names the account; else the account forgotten, and
/// the connection held as it, if any, let go.
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
    if record.account(&frame.name).is_none() {
        return Ok(Frame::NotFound);
    }
    if in_use::daemon_account(&mut tx, &record.name, &frame.name).await? {
        return Ok(Frame::InUse);
    }
    if !providers_daemons::accounts::delete(&mut tx, record.id, &frame.name).await? {
        return Ok(Frame::NotFound);
    }
    tx.commit().await?;
    daemon.live.remove_account(&record.name, &frame.name).await;
    daemon.live.changed(Kind::ProvidersDaemons);
    Ok(Frame::Deleted)
}
