//! Admitting an identity to a tool: to see it from its provider, to
//! join it, or both.

use diverge_sdk::daemon::endpoints::tools::Admits;
use diverge_sdk::daemon::endpoints::tools::admit::client::request;
use diverge_sdk::daemon::endpoints::tools::admit::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, key};
use crate::serve::reply;
use crate::store::tools::admissions;
use crate::store::{self, tools};

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
/// for a tool the grants do not reach; `Connected` for a connected
/// tool, whose runner admits; `Exists` for an identity admitted
/// already; else the admission on the tool, with the key minted when
/// it admits a connect, answered here and never again.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Admit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    let attached = agents_of(&mut tx, tool.id).await?;
    if !judge::tools::over(&standing, Over::Admit, &tool, active(daemon, tool.id).await, &attached) {
        return Ok(Frame::Forbidden);
    }
    if tool.is_connected() {
        return Ok(Frame::Connected);
    }
    let minted = match frame.admission.admits {
        Admits::List => None,
        Admits::Connect | Admits::Both => Some(key::mint()),
    };
    let hash = minted.as_deref().map(key::hash);
    match admissions::create(&mut tx, tool.id, &frame.admission, hash.as_deref()).await? {
        admissions::Created::Created => {}
        admissions::Created::Exists => return Ok(Frame::Exists),
    }
    tx.commit().await?;
    Ok(Frame::Admitted(minted))
}
