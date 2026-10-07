//! Getting one tool template.

use diverge_sdk::daemon::grant::tools_templates::Over;
use diverge_sdk::daemon::endpoints::tools::templates::get::client::request;
use diverge_sdk::daemon::endpoints::tools::templates::get::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, tools_templates};
use super::in_use;

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
    if !judge::tools_templates::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    let Some(record) = tools_templates::by_id(&mut conn, &frame.id, false).await? else {
        return Ok(Frame::NotFound);
    };
    let held = store::in_use::tools_templates(&mut conn).await?;
    if !judge::tools_templates::over(&standing, Over::Get, &record, in_use(&held, &record)) {
        return Ok(Frame::Forbidden);
    }
    Ok(Frame::Found(record.report()))
}
