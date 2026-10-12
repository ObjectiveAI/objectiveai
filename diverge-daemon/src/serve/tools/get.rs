//! Getting one tool.

use diverge_sdk::daemon::endpoints::tools::get::client::request;
use diverge_sdk::daemon::endpoints::tools::get::server::response::Frame;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Found, active, agents_of, report, report_dependency, report_service, resolve, service_agents, service_owner};
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
/// reports it — a record, a transient dependency that runs now, or a
/// service.
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
        Some(Found::Service { service, run }) => {
            let attached = service_agents(&mut conn, &service).await?;
            if !judge::tools::over(&standing, Over::Get, &Facts::service(&service, run.is_some(), &attached)) {
                return Ok(Frame::Forbidden);
            }
            let Some(owner) = service_owner(&mut conn, &service).await? else {
                return Ok(Frame::NotFound);
            };
            let running = run.as_ref().and_then(|run| run.container.clone());
            Ok(Frame::Found(report_service(&service, owner, attached, run.is_some(), running)))
        }
    }
}
