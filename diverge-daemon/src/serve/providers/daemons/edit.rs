//! Replacing a daemon record's mode or its links.

use diverge_sdk::daemon::endpoints::providers::daemons::edit::client::request;
use diverge_sdk::daemon::endpoints::providers::daemons::edit::server::response::Frame;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::add::linked;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
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
/// for a record the grants do not reach; `NoProvider` for a new link
/// whose provider is none on record; else the mode and the links as
/// the request states, each replaced whole where given and left where
/// not. A connection this daemon holds now is not dropped; the next
/// one opened presents the new credential, through the new links.
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
    if let Some(links) = &frame.links
        && !linked(&mut tx, links).await?
    {
        return Ok(Frame::NoProvider);
    }
    let mode = frame.mode.unwrap_or(record.mode);
    let links = frame.links.unwrap_or(record.links);
    providers_daemons::update(&mut tx, record.id, &mode, &links).await?;
    tx.commit().await?;
    daemon.live.changed(Kind::ProvidersDaemons);
    Ok(Frame::Edited)
}
