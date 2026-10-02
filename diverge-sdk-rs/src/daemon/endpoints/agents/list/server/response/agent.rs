//! One agent, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::agents::logs::server::response::Provider;

/// One agent of the caller's: what the daemon knows of it without
/// reading its log.
///
/// Everything here the daemon holds for the agent's life, so a list
/// costs no more than the agents it names. What an agent has SAID is
/// its [`logs`](crate::daemon::endpoints::agents::logs), read separately;
/// what it was made from is its template's, named by id, and its
/// mounts are the create's, and neither is repeated here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    /// The name, as its create gave it.
    pub name: String,
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
    /// Who made it, and through whom: the chain
    /// [`creator`](crate::daemon::creator) describes, the client
    /// first and what made this agent last. Never empty; one link
    /// for an agent the caller made with a
    /// [`create`](crate::daemon::endpoints::agents::create).
    pub creator: Vec<Creator>,
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
    /// The names of the [`tools`](crate::daemon::endpoints::tools)
    /// attached to it, in the order they were attached; empty for an
    /// agent with none. The tools list reports the same attachments
    /// from the other side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// Its tags, sorted bytewise: what [`tag`](crate::daemon::endpoints::agents::tag) put on it and
    /// [`untag`](crate::daemon::endpoints::agents::untag) has not taken off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
