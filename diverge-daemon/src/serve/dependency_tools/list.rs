//! Listing dependency tools, and keeping the list.

use std::collections::HashMap;
use std::future::Future;

use diverge_sdk::daemon::endpoints::dependency_tools::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::dependency_tools::list::server::response::{Attached, Frame, Listed};
use diverge_sdk::daemon::grant::dependency_tools::Over;
use diverge_sdk::daemon::key;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Loaded, run_of};
use crate::containers::ToolKey;
use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Source};
use crate::serve::reply;
use crate::store::{self, DependencyToolId, agents, dependency_tools};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every tool any
/// `list` grant reaches that the request's filter lets through, oldest
/// first, the first `count` of them, one frame each, then the word
/// that the list is whole — and from then on each tool added, changed
/// or removed as registrations make tools and reset what agents depend
/// on, as tools are tagged and deleted, and as their containers start
/// and stop, until the client cancels. The agents and the attachments
/// are one load each for the whole list, at every reading; a per-agent
/// tool whose agent is gone is left out.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::dependency_tools::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Tools {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::DependencyTools, Kind::StandaloneTools], frame.count, &source, Frame::from)
    .await
}

/// The tools as the caller may list them now.
struct Tools<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Tools<'_> {
    type Key = (String, key::Owner);
    type Item = Listed;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<((String, key::Owner), Listed)>, store::Error>> + Send {
        async move {
            let (all, rows, attached) = {
                let mut conn = self.daemon.store.acquire().await?;
                let all = dependency_tools::all(&mut conn).await?;
                let rows: HashMap<_, _> = agents::all(&mut conn).await?.into_iter().map(|agent| (agent.id, agent)).collect();
                let mut attached: HashMap<DependencyToolId, Vec<Attached>> = HashMap::new();
                for attachment in dependency_tools::attachments::all(&mut conn).await? {
                    if let Some(agent) = rows.get(&attachment.agent) {
                        attached.entry(attachment.tool).or_default().push(Attached {
                            agent: agent.key(),
                            declared: attachment.declared,
                        });
                    }
                }
                (all, rows, attached)
            };
            let runs: HashMap<_, _> = self
                .daemon
                .live
                .tool_runs()
                .await
                .into_iter()
                .filter_map(|run| match run.id {
                    ToolKey::Dependency(id) => Some((id, run_of(&run))),
                    ToolKey::Record(_) => None,
                })
                .collect();
            let busy = self.daemon.live.busy_tools().await;
            let mut listed = Vec::new();
            for tool in &all {
                let Some(owner) = tool.owner(tool.owner_agent.and_then(|id| rows.get(&id))) else {
                    continue;
                };
                let loaded = Loaded {
                    owner,
                    attached: attached.get(&tool.id).cloned().unwrap_or_default(),
                    run: runs.get(&tool.id).cloned(),
                    active: busy.contains(&ToolKey::Dependency(tool.id)),
                };
                let facts = loaded.facts(tool);
                if !judge::dependency_tools::over(self.standing, Over::List, &facts) || !filter::dependency_tools::test(self.filter, &facts) {
                    continue;
                }
                listed.push(((tool.template.clone(), loaded.owner.clone()), loaded.report(tool)));
            }
            Ok(listed)
        }
    }
}
