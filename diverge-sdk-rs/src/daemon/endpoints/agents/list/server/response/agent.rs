//! One agent, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator;
use crate::daemon::creator::Creator;
use crate::daemon::key;
use crate::daemon::endpoints::agents::logs::server::response::Provider;

/// One agent of the caller's: what the daemon knows of it without
/// reading its log.
///
/// Everything here the daemon holds for the agent's life, so a list
/// costs no more than the agents it names. What an agent has SAID is
/// its [`logs`](crate::daemon::endpoints::agents::logs), read
/// separately; what it was made from is its template's, named by id,
/// and its mounts are the create's, and neither is repeated here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    /// The name, as its create gave it, if it gave one; absent for an
    /// agent made with none, which is reached by its template and its
    /// index alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The template it was made from, by id: the hash a
    /// [`templates::create`](crate::daemon::endpoints::agents::templates::create)
    /// answered, and what a
    /// [`templates::list`](crate::daemon::endpoints::agents::templates::list)
    /// names it by.
    pub template: String,
    /// Its number among all agents of the caller's ever made from
    /// that template, deleted ones included: the first made is `1`,
    /// each after is one more, and no number is given twice. The
    /// template and the index together name the agent once and for
    /// all, where a name is free again once the agent is deleted.
    pub index: u64,
    /// Who made it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker; the maker's own maker is on the maker's list item.
    pub creator: Creator,
    /// The agent its create named as deployer, if any, as that agent
    /// was: its template, its index and its name, stable past its
    /// deletion. See the create's `deployer_agent`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployer_agent: Option<creator::Agent>,
    /// When the create made it.
    pub created: DateTime<Utc>,
    /// Whether the agent is active now: a loop is running in it.
    pub active: bool,
    /// When its activity last changed: the time it became active, if
    /// it is; the time it last ceased to be, if it is not; absent
    /// for an agent that has never been active. The `created` of its
    /// latest `active` or `inactive` log item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
    /// The provider it runs on, if active, or last ran on; absent
    /// for an agent that has never been active. See [`Provider`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// The `logs_index` of its latest log item: how long its log is,
    /// and where a read that wants only what comes next starts.
    /// `0` for an empty log.
    pub logs_index: u64,
    /// The [`tools`](crate::daemon::endpoints::tools) attached to it,
    /// each by its [`key`](crate::daemon::key) — its origin and its
    /// index, with its name beside when it has one — in the order
    /// they were attached; empty for an agent with none. The tools
    /// list reports the same attachments from the other side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<key::Tool>,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::agents::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::agents::untag) has not taken
    /// off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
