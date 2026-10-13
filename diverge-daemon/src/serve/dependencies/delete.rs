//! Deleting a dependency template.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::dependencies::delete::client::request;
use diverge_sdk::daemon::endpoints::dependencies::delete::server::response::Frame;
use diverge_sdk::daemon::grant::dependencies::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{agents_of, runs_of};
use crate::containers::services;
use crate::daemon::{Daemon, Kind};
use crate::database;
use crate::judge::filter::dependencies::Facts;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, dependencies, scopes};

/// Answer the delete and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `delete` grant at all; `NotFound`; `Forbidden`
/// for a template the grants do not reach; `InUse` while any tool runs
/// from it, transient or a service; else the template deleted — every
/// service made from it gone and retired, no agent depending on it any
/// more, every database scope ever made for a tool of it dropped with
/// its every table and taken off record, and its row kept, so one
/// declared anew is this one again, with fresh scopes.
async fn serve(frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::dependencies::holds(&standing, Over::Delete) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = dependencies::by_id(&mut tx, &frame.id, true).await? else {
        return Ok(Frame::NotFound);
    };
    let agents = agents_of(&mut tx, &record.id).await?;
    let runs = runs_of(daemon, &record.id).await;
    let facts = Facts {
        record: &record,
        agents: &agents,
        runs: &runs,
    };
    if !judge::dependencies::over(&standing, Over::Delete, &facts) {
        return Ok(Frame::Forbidden);
    }
    if !runs.is_empty() {
        return Ok(Frame::InUse);
    }
    let removed = store::services::delete_of_template(&mut tx, &record.id).await?;
    dependencies::agents::remove_template(&mut tx, &record.id).await?;
    for role in scopes::of_template(&mut tx, &record.id).await? {
        database::provision::drop(&mut tx, &role).await?;
        scopes::remove(&mut tx, &role).await?;
    }
    dependencies::delete(&mut tx, &record.id).await?;
    tx.commit().await?;
    services::template_deleted(daemon, removed).await;
    daemon.live.changed(Kind::Dependencies);
    daemon.live.changed(Kind::Tools);
    Ok(Frame::Deleted)
}
