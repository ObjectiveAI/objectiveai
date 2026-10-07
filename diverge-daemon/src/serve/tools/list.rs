//! Listing tools.

use std::collections::HashMap;

use diverge_sdk::daemon::endpoints::tools::Admission;
use diverge_sdk::daemon::endpoints::tools::list::client::request;
use diverge_sdk::daemon::endpoints::tools::list::server::response::Frame;
use diverge_sdk::daemon::endpoints::tools::routes::Path;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::daemon::key;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::reply;
use crate::store::tools::admissions;
use crate::store::{self, ToolId, agents, routes, tools};

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every tool any
/// `list` grant reaches that the request's filter lets through, oldest
/// created first, at most `count` of them, one frame each — and
/// nothing at all, which is an answer, when none does. The
/// attachments, the routes and the admissions are one load each for
/// the whole list.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::tools::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let all = tools::all(&mut conn).await?;
    let keys: HashMap<_, _> = agents::all(&mut conn).await?.into_iter().map(|agent| (agent.id, agent.key())).collect();
    let mut attached: HashMap<ToolId, Vec<key::Agent>> = HashMap::new();
    for attachment in tools::attachments::all(&mut conn).await? {
        if let Some(key) = keys.get(&attachment.agent) {
            attached.entry(attachment.tool).or_default().push(key.clone());
        }
    }
    let mut routed: HashMap<ToolId, Vec<Path>> = HashMap::new();
    for route in routes::all(&mut conn).await? {
        routed.entry(route.tool).or_default().push(route.path());
    }
    let mut admitted: HashMap<ToolId, Vec<Admission>> = HashMap::new();
    for record in admissions::all(&mut conn).await? {
        admitted.entry(record.tool).or_default().push(record.admission);
    }
    drop(conn);
    let states = daemon.live.active_tools().await;
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let empty = Vec::new();
    let sent = all
        .iter()
        .map(|tool| (tool, states.contains_key(&tool.id), attached.get(&tool.id).unwrap_or(&empty)))
        .filter(|(tool, active, attached)| judge::tools::over(&standing, Over::List, tool, *active, attached))
        .filter(|(tool, active, attached)| filter::tools::test(&frame.filter, tool, *active, attached))
        .take(cap);
    for (tool, active, attached) in sent {
        let routes = routed.get(&tool.id).cloned().unwrap_or_default();
        let admissions = if tool.is_connected() {
            Vec::new()
        } else {
            admitted.get(&tool.id).cloned().unwrap_or_default()
        };
        reply::reply(
            scope,
            &Frame::Tool(tool.report(active, states.get(&tool.id).cloned().flatten(), attached.clone(), routes, admissions)),
        )
        .await;
    }
    Ok(())
}
