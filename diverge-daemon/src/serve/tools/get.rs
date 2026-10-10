//! Getting one tool.

use diverge_sdk::daemon::endpoints::tools::get::client::request;
use diverge_sdk::daemon::endpoints::tools::get::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Found, active, agents_of, report, report_dependency, resolve};
use crate::daemon::Daemon;
use crate::judge::filter::tools::Facts;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Answer the get and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `get` grant at all; `NotFound`; `Forbidden`
/// for a tool the grants do not reach; else the tool as a list
/// reports it — a record, or a dependency that runs now.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::tools::holds(&standing, Over::Get) {
        return Ok(Frame::Forbidden);
    }
    match resolve(&mut conn, daemon, &frame.tool, false).await? {
        None => Ok(Frame::NotFound),
        Some(Found::Record(tool)) => {
            let attached = agents_of(&mut conn, tool.id).await?;
            if !judge::tools::over(&standing, Over::Get, &Facts::record(&tool, active(daemon, tool.id).await, &attached)) {
                return Ok(Frame::Forbidden);
            }
            Ok(Frame::Found(report(&mut conn, daemon, &tool).await?))
        }
        Some(Found::Dependency(run)) => {
            let Some(facts) = Facts::dependency(&run) else {
                return Ok(Frame::NotFound);
            };
            if !judge::tools::over(&standing, Over::Get, &facts) {
                return Ok(Frame::Forbidden);
            }
            Ok(match report_dependency(&run) {
                Some(item) => Frame::Found(item),
                None => Frame::NotFound,
            })
        }
    }
}
