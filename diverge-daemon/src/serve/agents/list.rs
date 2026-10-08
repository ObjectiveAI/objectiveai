//! Listing agents, and keeping the list.

use std::collections::HashMap;
use std::future::Future;

use diverge_sdk::daemon::endpoints::agents::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::agents::list::server::response::{Agent, Frame};
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::daemon::key;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::Failure;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::logs;
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{AgentId, agents, tools};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every agent any
/// `list` grant reaches that the request's filter lets through, oldest
/// created first, the first `count` of them, one frame each, then the
/// word that the list is whole — and from then on each agent added,
/// changed or removed as the records, the attachments and the runs
/// change, until the client cancels. The log's length is as of the
/// agent's last change of state, not of its last line: the log's own
/// growth is read on `agents logs`.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), Failure> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::agents::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Agents], frame.count, &source, |change| match change {
        Change::Added(agent) => Frame::Added(agent),
        Change::Changed(agent) => Frame::Changed(agent),
        Change::Removed(agent) => Frame::Removed(agent),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The agents as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = AgentId;
    type Item = Agent;
    type Error = Failure;

    fn read(&self) -> impl Future<Output = Result<Vec<(AgentId, Agent)>, Failure>> + Send {
        async move {
            let (all, attached) = {
                let mut conn = self.daemon.store.acquire().await?;
                let all = agents::all(&mut conn).await?;
                let keys: HashMap<_, _> = tools::all(&mut conn).await?.into_iter().map(|tool| (tool.id, tool.key())).collect();
                let mut attached: HashMap<AgentId, Vec<key::Tool>> = HashMap::new();
                for attachment in tools::attachments::all(&mut conn).await? {
                    if let Some(key) = keys.get(&attachment.tool) {
                        attached.entry(attachment.agent).or_default().push(key.clone());
                    }
                }
                (all, attached)
            };
            let actives = self.daemon.live.active_agents().await;
            let mut listed = Vec::new();
            for agent in &all {
                let active = actives.contains(&agent.id);
                if !judge::agents::over(self.standing, Over::List, agent, active) || !filter::agents::test(self.filter, agent, active) {
                    continue;
                }
                let logs_index = logs::count(&self.daemon.logs, agent.id).await?;
                let tools = attached.get(&agent.id).cloned().unwrap_or_default();
                listed.push((agent.id, agent.report(active, tools, logs_index)));
            }
            Ok(listed)
        }
    }
}
