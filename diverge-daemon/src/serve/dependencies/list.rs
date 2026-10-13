//! Listing dependency templates, and keeping the list.

use std::collections::HashMap;
use std::future::Future;

use diverge_sdk::daemon::endpoints::dependencies::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::dependencies::list::server::response::{Frame, Listed, Run};
use diverge_sdk::daemon::grant::dependencies::Over;
use diverge_sdk::daemon::key;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{run_of, run_order};
use crate::daemon::{Daemon, Kind};
use crate::judge::filter::dependencies::Facts;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Change, Source};
use crate::serve::reply;
use crate::store::{self, agents, dependencies};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every template
/// any `list` grant reaches that the request's filter lets through,
/// oldest first, the first `count` of them, one frame each, then the
/// word that the list is whole — and from then on each template added,
/// changed or removed as registrations make templates and reset what
/// agents depend on, as templates are tagged and deleted, and as the
/// tools running from them start and stop, until the client cancels.
/// What agents depend on is one load for the whole list, at every
/// reading.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::dependencies::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Templates {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::Dependencies, Kind::Tools], frame.count, &source, |change| match change {
        Change::Added(template) => Frame::Added(template),
        Change::Changed(template) => Frame::Changed(template),
        Change::Removed(template) => Frame::Removed(template),
        Change::Listed => Frame::Listed,
    })
    .await
}

/// The templates as the caller may list them now.
struct Templates<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Templates<'_> {
    type Key = String;
    type Item = Listed;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(String, Listed)>, store::Error>> + Send {
        async move {
            let (all, depended) = {
                let mut conn = self.daemon.store.acquire().await?;
                let all = dependencies::all(&mut conn).await?;
                let keys: HashMap<_, _> = agents::all(&mut conn).await?.into_iter().map(|agent| (agent.id, agent.key())).collect();
                let mut depended: HashMap<String, Vec<key::Agent>> = HashMap::new();
                for dependence in dependencies::agents::all(&mut conn).await? {
                    if let Some(key) = keys.get(&dependence.agent) {
                        depended.entry(dependence.template).or_default().push(key.clone());
                    }
                }
                (all, depended)
            };
            let mut live = self.daemon.live.tool_runs().await;
            live.sort_by_key(|run| run_order(run));
            let mut runs: HashMap<String, Vec<Run>> = HashMap::new();
            for (template, run) in live.iter().filter_map(|run| run_of(run)) {
                runs.entry(template).or_default().push(run);
            }
            let none_agents = Vec::new();
            let none_runs = Vec::new();
            let mut listed = Vec::new();
            for record in &all {
                let agents = depended.get(&record.id).unwrap_or(&none_agents);
                let runs = runs.get(&record.id).unwrap_or(&none_runs);
                let facts = Facts { record, agents, runs };
                if !judge::dependencies::over(self.standing, Over::List, &facts) || !filter::dependencies::test(self.filter, &facts) {
                    continue;
                }
                listed.push((record.id.clone(), record.report(agents.clone(), runs.clone())));
            }
            Ok(listed)
        }
    }
}
