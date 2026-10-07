//! Listing agents.

use std::collections::HashMap;

use diverge_sdk::daemon::endpoints::agents::list::client::request;
use diverge_sdk::daemon::endpoints::agents::list::server::response::Frame;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::daemon::key;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Failure, active};
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who, filter};
use crate::logs;
use crate::serve::reply;
use crate::store::{AgentId, agents, tools};

/// Send the list and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every agent any
/// `list` grant reaches that the request's filter lets through, oldest
/// created first, at most `count` of them, one frame each — and
/// nothing at all, which is an answer, when none does. The attachments
/// are one load for the whole list.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), Failure> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let all = agents::all(&mut conn).await?;
    let keys: HashMap<_, _> = tools::all(&mut conn).await?.into_iter().map(|tool| (tool.id, tool.key())).collect();
    let mut attached: HashMap<AgentId, Vec<key::Tool>> = HashMap::new();
    for attachment in tools::attachments::all(&mut conn).await? {
        if let Some(key) = keys.get(&attachment.tool) {
            attached.entry(attachment.agent).or_default().push(key.clone());
        }
    }
    drop(conn);
    let cap = frame.count.map_or(usize::MAX, |count| usize::try_from(count).unwrap_or(usize::MAX));
    let sent = all
        .iter()
        .map(|agent| (agent, active(daemon, agent.id)))
        .filter(|(agent, active)| judge::agents::over(&standing, Over::List, agent, *active))
        .filter(|(agent, active)| filter::agents::test(&frame.filter, agent, *active))
        .take(cap);
    for (agent, active) in sent {
        let logs_index = logs::count(&daemon.logs, agent.id).await?;
        let tools = attached.get(&agent.id).cloned().unwrap_or_default();
        reply::reply(scope, &Frame::Agent(agent.report(active, tools, logs_index))).await;
    }
    Ok(())
}
