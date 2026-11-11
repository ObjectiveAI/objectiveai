//! One tool, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;
use crate::daemon::agent_log::Provider;
use crate::daemon::creator::Creator;
use crate::daemon::endpoints::standalone_tools::templates::Mode;
use crate::daemon::key;
use crate::shared::permission::Grant;

/// One tool of the caller's: what the daemon knows of it.
///
/// Everything here the daemon holds for the tool's life, so a list
/// costs no more than the tools it names. What a tool was made from
/// is its template's, named by id, and its mounts are the create's,
/// and neither is repeated here. A connected tool — another daemon's,
/// held here — is no tool of this family: it is listed by the
/// [`connected_standalone_tools`](crate::daemon::endpoints::connected_standalone_tools)
/// list, and nowhere here. Nor is a dependency tool — one running
/// from a template an agent's program declared — which is listed under
/// its template by the
/// [`dependency_tools`](crate::daemon::endpoints::dependency_tools)
/// list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Tool {
    /// The name, as its create gave it, if it gave one; absent for a
    /// tool made with none, which is reached by its template and its
    /// index alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The template it was made from, by id: the hash a
    /// [`templates::create`](crate::daemon::endpoints::standalone_tools::templates::create)
    /// answered.
    pub template: String,
    /// Its number among all tools of the caller's ever made from that
    /// template, deleted ones included: the first made is `1`, each
    /// after is one more, and no number is given twice. The template
    /// and the index together name the tool once and for all, where a
    /// name is free again once the tool is deleted.
    pub index: u64,
    /// How the tool is kept running, as its template's `mode` says:
    /// transient, run while used, or a service, kept while any agent is
    /// attached or any connect is open. See [`Mode`].
    pub mode: Mode,
    /// The provider the container runs on, if it runs, or last ran on;
    /// absent for a tool that has never run. See [`Provider`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// The container's id while it runs, as the provider's run answered
    /// it. Absent while the container does not run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Who made it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker; the maker's own maker is on the maker's list item.
    pub creator: Creator,
    /// When the create made it.
    pub created: DateTime<Utc>,
    /// Whether the tool is active now: working, a call or a read the
    /// daemon routed into it in flight — what its log marks `active`.
    /// Whether its container is up is [`deployed`](Self::deployed).
    pub active: bool,
    /// When that last changed: the time its first call or read in
    /// flight began, if one is; the time its last ended, if not;
    /// absent for a tool never asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
    /// Whether its container is up now.
    pub deployed: bool,
    /// When that last changed: the time it came up, if it is up; the
    /// time it last went down, if not; absent for one never deployed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_deployed: Option<DateTime<Utc>>,
    /// The agents it is attached to directly, by
    /// [`attach_agent_instance`](crate::daemon::endpoints::standalone_tools::attach_agent_instance),
    /// each by its [`key`](crate::daemon::key) — its template and its
    /// index, with its name beside when it has one — in the order
    /// they were attached; empty for none. The agents list reports
    /// the same attachments from the other side, beside those an
    /// agent has through its template.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attached_agent_instances: Vec<key::Agent>,
    /// The agent templates it is attached to, by
    /// [`attach_agent_template`](crate::daemon::endpoints::standalone_tools::attach_agent_template),
    /// each by its id, in the order they were attached; empty for
    /// none. Every agent of each has the tool.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attached_agent_templates: Vec<String>,
    /// What its container declared it may ask of the daemon, at its
    /// registration when it last started: see [`Grant`]. Declared anew
    /// at every start, so it changes as the image does. Absent when
    /// empty — a tool whose container declared none, or has never
    /// started.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Grant>,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::standalone_tools::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::standalone_tools::untag) has not taken
    /// off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Keyed by the template and the index, which together name the tool
/// once and for all; the name is not in it.
impl Keyed for Tool {
    type Key = (String, u64);

    fn key(&self) -> (String, u64) {
        (self.template.clone(), self.index)
    }
}
