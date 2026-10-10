//! Admitting an identity to a tool: to join it from its provider.

use diverge_sdk::daemon::endpoints::tools::admit::client::request;
use diverge_sdk::daemon::endpoints::tools::admit::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Found, READ_ONLY, reaches, resolve};
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, key};
use crate::serve::reply;
use crate::store::tools::admissions;
use crate::store;

/// Answer the admit and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `admit` grant at all; `NotFound`; `Forbidden`
/// for a tool the grants do not reach; the error for a dependency,
/// listed to nobody and joined by nobody; `Connected` for a connected
/// tool, whose runner admits; `Exists` for an identity admitted
/// already; else the admission on the tool, with the key minted for
/// it, answered here and never again.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Admit) {
        return Ok(Frame::Forbidden);
    }
    let Some(found) = resolve(&mut tx, daemon, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    if !reaches(&mut tx, daemon, &standing, Over::Admit, &found).await? {
        return Ok(Frame::Forbidden);
    }
    let Found::Record(tool) = found else {
        return Ok(Frame::Error(reply::failure(&READ_ONLY)));
    };
    if tool.is_connected() {
        return Ok(Frame::Connected);
    }
    let minted = key::mint();
    match admissions::create(&mut tx, tool.id, &frame.admission, &key::hash(&minted)).await? {
        admissions::Created::Created => {}
        admissions::Created::Exists => return Ok(Frame::Exists),
    }
    tx.commit().await?;
    daemon.live.changed(Kind::Tools);
    Ok(Frame::Admitted(minted))
}
