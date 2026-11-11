//! One connected tool, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;
use crate::daemon::creator::Creator;
use crate::daemon::endpoints::daemons::DaemonId;
use crate::daemon::key;
use crate::daemon::reference;

use super::Key;

/// One connected tool of the caller's: what the daemon knows of it.
///
/// Everything here the daemon holds for the tool's life, so a list
/// costs no more than the tools it names. What the tool is made from —
/// its image, its limits, its mounts, its mode, the grants its
/// container declares — is the other daemon's, and unknown here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct ConnectedStandaloneTool {
    /// The name, as its create gave it, if it gave one; absent for a
    /// tool made with none, which is reached by its daemon, its tool
    /// there and its index alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The daemon the tool is on, by the ID of this daemon's record of
    /// it, as the create named it — the first of the route's daemons,
    /// where it has a route.
    pub daemon: DaemonId,
    /// The remote-daemon records the tool's connect is relayed through
    /// beyond `daemon`, as the create named them: see
    /// [`create`](crate::daemon::endpoints::connected_standalone_tools::create).
    /// Absent when empty: the tool is on `daemon`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route: Vec<DaemonId>,
    /// The tool, as that daemon names it and as the create named it:
    /// see [`reference::Tool`].
    pub tool: reference::Tool,
    /// Its number among all connected tools of the caller's ever held
    /// from that daemon's tool, deleted ones included: the first is
    /// `1`, each after is one more, and no number is given twice. The
    /// daemon, the tool and the index together name the connected tool
    /// once and for all, where a name is free again once the tool is
    /// deleted.
    pub index: u64,
    /// Who made it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker; the maker's own maker is on the maker's list item.
    pub creator: Creator,
    /// When the create made it.
    pub created: DateTime<Utc>,
    /// Whether the tool is active now: a call or a read the daemon
    /// routed into it in flight — what its log marks `active`. Whether
    /// its connect is held is [`deployed`](Self::deployed).
    pub active: bool,
    /// When that last changed: the time its first call or read in
    /// flight began, if one is; the time its last ended, if not;
    /// absent for a tool never asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
    /// Whether it is deployed now: the daemon holds the other daemon's
    /// connect on it.
    pub deployed: bool,
    /// When that last changed: the time the connect opened, if it is
    /// held; the time it was last let go, if not; absent for one never
    /// deployed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_deployed: Option<DateTime<Utc>>,
    /// The agents it is attached to directly, by
    /// [`attach_agent_instance`](crate::daemon::endpoints::connected_standalone_tools::attach_agent_instance),
    /// each by its [`key`](crate::daemon::key) — its template and its
    /// index, with its name beside when it has one — in the order
    /// they were attached; empty for none. The agents list reports
    /// the same attachments from the other side, beside those an
    /// agent has through its template.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attached_agent_instances: Vec<key::Agent>,
    /// The agent templates it is attached to, by
    /// [`attach_agent_template`](crate::daemon::endpoints::connected_standalone_tools::attach_agent_template),
    /// each by its id, in the order they were attached; empty for
    /// none. Every agent of each has the tool.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attached_agent_templates: Vec<String>,
    /// Its tags, sorted bytewise: what
    /// [`tag`](crate::daemon::endpoints::connected_standalone_tools::tag)
    /// put on it and
    /// [`untag`](crate::daemon::endpoints::connected_standalone_tools::untag)
    /// has not taken off. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Keyed by the daemon, the tool there and the index, which together
/// name the connected tool once and for all: see [`Key`]. The name is
/// not in it, nor the route, which the three fix.
impl Keyed for ConnectedStandaloneTool {
    type Key = Key;

    fn key(&self) -> Key {
        Key {
            daemon: self.daemon,
            tool: self.tool.clone(),
            index: self.index,
        }
    }
}
