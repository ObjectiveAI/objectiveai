//! Taking an admission off a tool.

use diverge_sdk::daemon::endpoints::tools::unadmit::client::request;
use diverge_sdk::daemon::endpoints::tools::unadmit::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{active, agents_of};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::tools::admissions;
use crate::store::{self, tools};

/// Answer the unadmit and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `unadmit` grant at all; `NotFound`;
/// `Forbidden` for a tool the grants do not reach; else no admission
/// for the identity on the tool from then on, whether or not one was.
/// A connector joined through the admission's key now is not dropped;
/// the next is refused.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut tx = daemon.store.begin().await?;
    let Some(standing) = Standing::of(&mut tx, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Unadmit) {
        return Ok(Frame::Forbidden);
    }
    let Some(tool) = tools::by_reference(&mut tx, &frame.tool, true).await? else {
        return Ok(Frame::NotFound);
    };
    let attached = agents_of(&mut tx, tool.id).await?;
    if !judge::tools::over(&standing, Over::Unadmit, &tool, active(daemon, tool.id), &attached) {
        return Ok(Frame::Forbidden);
    }
    admissions::delete(&mut tx, tool.id, &frame.identity).await?;
    tx.commit().await?;
    Ok(Frame::Unadmitted)
}
