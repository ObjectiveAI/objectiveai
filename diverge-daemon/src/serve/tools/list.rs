//! Listing tools, and keeping the list.

use std::collections::HashMap;
use std::future::Future;

use diverge_sdk::daemon::endpoints::tools::Admission;
use diverge_sdk::daemon::endpoints::tools::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::tools::list::server::response::{Frame, Tool};
use diverge_sdk::daemon::endpoints::tools::routes::Path;
use diverge_sdk::daemon::grant::tools::Over;
use diverge_sdk::daemon::key;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::tools::admissions;
use crate::store::{self, ToolId, agents, routes, tools};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every tool any
/// `list` grant reaches that the request's filter lets through, oldest
/// created first, the first `count` of them, one frame each, then the
/// word that the list is whole — and from then on each tool added,
/// changed or removed as the records, the attachments, the routes,
/// the admissions and the runs change, until the client cancels. The
/// attachments, the routes and the admissions are one load each for
/// the whole list, at every reading.
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
    type Key = ToolId;
    type Item = Tool;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(ToolId, Tool)>, store::Error>> + Send {
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
            let mut routed: HashMap<ToolId, Vec<Path>> = HashMap::new();
            for route in routes::all(&mut conn).await? {
                routed.entry(route.tool).or_default().push(route.path());
            }
            let mut admitted: HashMap<ToolId, Vec<Admission>> = HashMap::new();
            for record in admissions::all(&mut conn).await? {
                admitted.entry(record.tool).or_default().push(record.admission);
            }
            drop(conn);
            let states = self.daemon.live.active_tools().await;
            let empty = Vec::new();
            let mut listed = Vec::new();
            for tool in &all {
                let active = states.contains_key(&tool.id);
                let attached = attached.get(&tool.id).unwrap_or(&empty);
                if !judge::tools::over(self.standing, Over::List, tool, active, attached) || !filter::tools::test(self.filter, tool, active, attached) {
                    continue;
                }
                let routes = routed.get(&tool.id).cloned().unwrap_or_default();
                let admissions = if tool.is_connected() {
                    Vec::new()
                } else {
                    admitted.get(&tool.id).cloned().unwrap_or_default()
                };
                let running = states.get(&tool.id).cloned().flatten();
                listed.push((tool.id, tool.report(active, running, attached.clone(), routes, admissions)));
            }
            Ok(listed)
        }
    }
}
