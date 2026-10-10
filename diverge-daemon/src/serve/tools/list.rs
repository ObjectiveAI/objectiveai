//! Listing tools, and keeping the list.

use std::collections::HashMap;
use std::future::Future;

use diverge_sdk::daemon::endpoints::tools::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::tools::list::server::response::{Frame, Tool};
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::daemon::key;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::report_dependency;
use crate::containers::ToolKey;
use crate::daemon::{Daemon, Kind};
use crate::judge::filter::tools::Facts;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, ToolId, agents, tools};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every tool any
/// `list` grant reaches that the request's filter lets through —
/// every record, oldest created first, and every dependency running
/// now, oldest deployed first after them — the first `count` of
/// them, one frame each, then the word that the list is whole — and
/// from then on each tool added, changed or removed as the records,
/// the attachments and the runs change, a dependency added when its
/// agent's container starts and removed when it ends, until the
/// client cancels. The attachments are one load for the whole list,
/// at every reading.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::tools::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Tools], frame.count, &source, |change| match change {
        Change::Added(tool) => Frame::Added(tool),
        Change::Changed(tool) => Frame::Changed(tool),
        Change::Removed(tool) => Frame::Removed(tool),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The tools as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = ToolKey;
    type Item = Tool;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(ToolKey, Tool)>, store::Error>> + Send {
        async move {
            let mut conn = self.daemon.store.acquire().await?;
            let all = tools::all(&mut conn).await?;
            let keys: HashMap<_, _> = agents::all(&mut conn).await?.into_iter().map(|agent| (agent.id, agent.key())).collect();
            let mut attached: HashMap<ToolId, Vec<key::Agent>> = HashMap::new();
            for attachment in tools::attachments::all(&mut conn).await? {
                if let Some(key) = keys.get(&attachment.agent) {
                    attached.entry(attachment.tool).or_default().push(key.clone());
                }
            }
            drop(conn);
            let states = self.daemon.live.active_tools().await;
            let empty = Vec::new();
            let mut listed = Vec::new();
            for tool in &all {
                let key = ToolKey::Record(tool.id);
                let active = states.contains_key(&key);
                let attached = attached.get(&tool.id).unwrap_or(&empty);
                let facts = Facts::record(tool, active, attached);
                if !judge::tools::over(self.standing, Over::List, &facts) || !filter::tools::test(self.filter, &facts) {
                    continue;
                }
                let running = states.get(&key).cloned().flatten();
                listed.push((key, tool.report(active, running, attached.clone())));
            }
            let mut dependencies: Vec<_> = self
                .daemon
                .live
                .tool_runs()
                .await
                .into_iter()
                .filter(|run| run.dependency.is_some())
                .collect();
            dependencies.sort_by_key(|run| run.dependency.as_ref().map(|dependency| (dependency.started, dependency.id)));
            for run in dependencies {
                let Some(facts) = Facts::dependency(&run) else {
                    continue;
                };
                if !judge::tools::over(self.standing, Over::List, &facts) || !filter::tools::test(self.filter, &facts) {
                    continue;
                }
                if let Some(item) = report_dependency(&run) {
                    listed.push((run.id, item));
                }
            }
            Ok(listed)
        }
    }
}
