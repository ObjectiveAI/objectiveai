//! Getting one dependency template.

use diverge_sdk::daemon::endpoints::dependencies::get::client::request;
use diverge_sdk::daemon::endpoints::dependencies::get::server::response::Frame;
use diverge_sdk::daemon::grant::dependencies::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{agents_of, reaches, runs_of};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, dependencies};

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `get` grant at all; `NotFound`; `Forbidden` for
/// a template the grants do not reach; else the template as a list
/// reports it.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::dependencies::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = dependencies::by_id(&mut conn, &frame.id, false).await? else {
        return Ok(Frame::NotFound);
    };
    if !reaches(&mut conn, daemon, &standing, Over::Get, &record).await? {
        return Ok(Frame::Forbidden);
    }
    let agents = agents_of(&mut conn, &record.id).await?;
    let runs = runs_of(daemon, &record.id).await;
    Ok(Frame::Found(record.report(agents, runs)))
}
