//! Replacing a daemon record's links, by a patch.

use diverge_sdk::daemon::endpoints::providers::daemons::edit::client::request::{self, Editable};
use diverge_sdk::daemon::endpoints::providers::daemons::edit::server::response::Frame;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::add::linked;
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

/// `Forbidden` with no `edit` grant at all; `NotFound`; `Forbidden`
/// for a record the grants do not reach; the `patch` error for a
/// patch that could not be applied or a result that is not the
/// record's links; `NoProvider` for a link whose provider is none on
/// record; else the links as the result states. A connection this
/// daemon holds now is not dropped; the next one opened goes through
/// the new links.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::providers_daemons::holds(&standing, Over::Edit) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = providers_daemons::by_name(&mut tx, &frame.name, true).await? else {
        return Ok(Frame::NotFound);
    };
    let connected = daemon.live.is_daemon_connected(&record.name).await;
    if !judge::providers_daemons::over(&standing, Over::Edit, &record, connected) {
        return Ok(Frame::Forbidden);
    }
    let current = Editable {
        links: record.links.clone(),
    };
    let result = match inner::patched(&current, &frame.patch) {
        Ok(result) => result,
        Err(what) => return Ok(Frame::Error(reply::patch(&what))),
    };
    if result.links != current.links && !linked(&mut tx, &result.links).await? {
        return Ok(Frame::NoProvider);
    }
    providers_daemons::update(&mut tx, record.id, &result.links).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersDaemons);
    Ok(Frame::Edited)
}
