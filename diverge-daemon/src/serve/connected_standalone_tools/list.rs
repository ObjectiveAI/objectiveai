//! Listing connected tools, and keeping the list.

use std::collections::HashMap;
use std::future::Future;

use diverge_sdk::daemon::endpoints::connected_standalone_tools::list::client::request::{self, Filter};
use diverge_sdk::daemon::endpoints::connected_standalone_tools::list::server::response::{ConnectedStandaloneTool, Frame};
use diverge_sdk::daemon::grant::connected_standalone_tools::Over;
use diverge_sdk::daemon::key;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::containers::ToolKey;
use crate::daemon::{Daemon, Kind};
use crate::judge::filter::connected_standalone_tools::Facts;
use crate::judge::{self, Standing, Who, filter};
use crate::serve::stream::{self, Source};
use crate::serve::reply;
use crate::store::{self, ConnectedToolId, agents, connected_standalone_tools};

/// Send the list and its changes, then finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// One `Forbidden` with no `list` grant at all; else every record any
/// `list` grant reaches that the request's filter lets through, oldest
/// created first, the first `count` of them, one frame each, then the
/// word that the list is whole — and from then on each tool added,
/// changed or removed as the records, the attachments and the connects
/// change, until the client cancels. The attachments are one load for
/// the whole list, at every reading; an agent's name changing changes
/// the tool's item, so the agents' word is heard too.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::connected_standalone_tools::holds(&standing, Over::List) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let source = Listed {
        daemon,
        standing: &standing,
        filter: &frame.filter,
    };
    stream::listing(scope, daemon, &[Kind::ConnectedStandaloneTools, Kind::Agents], frame.count, &source, Frame::from)
    .await
}

/// The connected tools as the caller may list them now.
struct Listed<'a> {
    daemon: &'a Daemon,
    standing: &'a Standing,
    filter: &'a Filter,
}

impl Source for Listed<'_> {
    type Key = ConnectedToolId;
    type Item = ConnectedStandaloneTool;
    type Error = store::Error;

    fn read(&self) -> impl Future<Output = Result<Vec<(ConnectedToolId, ConnectedStandaloneTool)>, store::Error>> + Send {
        async move {
            let mut conn = self.daemon.store.acquire().await?;
            let all = connected_standalone_tools::all(&mut conn).await?;
            let keys: HashMap<_, _> = agents::all(&mut conn).await?.into_iter().map(|agent| (agent.id, agent.key())).collect();
            let mut attached: HashMap<ConnectedToolId, Vec<key::Agent>> = HashMap::new();
            for attachment in connected_standalone_tools::attachments::all(&mut conn).await? {
                if let Some(key) = keys.get(&attachment.agent) {
                    attached.entry(attachment.tool).or_default().push(key.clone());
                }
            }
            let mut templates: HashMap<ConnectedToolId, Vec<String>> = HashMap::new();
            for attachment in connected_standalone_tools::template_attachments::all(&mut conn).await? {
                templates.entry(attachment.tool).or_default().push(attachment.template);
            }
            drop(conn);
            let states = self.daemon.live.active_tools().await;
            let busy = self.daemon.live.busy_tools().await;
            let empty = Vec::new();
            let no_templates = Vec::new();
            let mut listed = Vec::new();
            for tool in &all {
                let key = ToolKey::Connected(tool.id);
                let deployed = states.contains_key(&key);
                let active = busy.contains(&key);
                let attached = attached.get(&tool.id).unwrap_or(&empty);
                let attached_templates = templates.get(&tool.id).unwrap_or(&no_templates);
                let facts = Facts {
                    tool,
                    active,
                    deployed,
                    attached_agent_instances: attached,
                    attached_agent_templates: attached_templates,
                };
                if !judge::connected_standalone_tools::over(self.standing, Over::List, &facts) || !filter::connected_standalone_tools::test(self.filter, &facts) {
                    continue;
                }
                listed.push((tool.id, tool.report(active, deployed, attached.clone(), attached_templates.clone())));
            }
            Ok(listed)
        }
    }
}
