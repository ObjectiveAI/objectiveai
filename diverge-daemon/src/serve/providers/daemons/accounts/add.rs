//! Adding an account to a daemon record.

use diverge_sdk::daemon::endpoints::providers::daemons::accounts::add::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::accounts::add::server::response::Frame;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, providers_daemons};

/// Answer the add and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `edit` grant over daemon records at all;
/// `NoDaemon`; `Forbidden` for a record the grants do not reach;
/// `Exists` for a name the record has an account under already; else
/// the account added, holding its mode.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_daemons::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = providers_daemons::by_name(&mut tx, &frame.daemon, true).await? else {
        return Ok(Frame::NoDaemon);
    };
    let connected = daemon.live.is_daemon_connected(&record.name).await;
    if !judge::providers_daemons::over(&standing, Over::Edit, &record, connected) {
        return Ok(Frame::Forbidden);
    }
    match providers_daemons::accounts::add(&mut tx, record.id, &frame.name, &frame.mode).await? {
        providers_daemons::accounts::Added::Added => {}
        providers_daemons::accounts::Added::Exists => return Ok(Frame::Exists),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersDaemons);
    Ok(Frame::Added)
}
