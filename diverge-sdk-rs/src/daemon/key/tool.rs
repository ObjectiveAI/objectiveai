//! One tool, once and for all.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::daemons::DaemonId;
use crate::daemon::reference;

use super::Owner;

/// One tool of the caller's, once and for all: a standalone tool on
/// record, by the template it was made from and its index; a connected
/// tool, by the daemon it is on, the tool there and its index — each
/// with its name beside when it has one; or a dependency tool, by the
/// template it runs from and whose it is. Externally tagged on the
/// wire, snake case: `{"record":{…}}`, `{"connected_standalone":{…}}`
/// or `{"dependency":{…}}`.
///
/// A connected tool is no record of the
/// [`standalone_tools`](crate::daemon::endpoints::standalone_tools)
/// family: it is another daemon's standalone tool, held here by a
/// [`connected_standalone_tools::create`](crate::daemon::endpoints::connected_standalone_tools::create),
/// and what names it once and for all is the daemon, the tool as that
/// daemon names it, and its index among all held from that tool.
///
/// A dependency tool is no record of either family: it runs from a
/// [dependency template](crate::daemon::endpoints::dependency_tools) an
/// agent's program declared — a transient one for the agent's run,
/// a service for as long as any agent is attached to it. What names
/// it once and for all is its template's id and its [`Owner`]: the
/// agent, itself once and for all, for a transient tool and for a
/// service whose template's `scope` is `instance`; the agent template
/// or everyone for the other two scopes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    /// A standalone tool on record, made by a create from a template.
    Record {
        /// The template it was made from, by id.
        template: String,
        /// Its number among all tools of the caller's ever made from
        /// that template, as its list item carries it.
        index: u64,
        /// Its name, as its create gave it, if it gave one. Absent when
        /// it has none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A connected tool: another daemon's standalone tool, held here.
    ConnectedStandalone {
        /// The daemon the tool is on, by the ID of this daemon's record
        /// of it, as the create named it.
        daemon: DaemonId,
        /// The tool, as that daemon names it and as the create named
        /// it: see [`reference::Tool`].
        tool: reference::Tool,
        /// Its number among all connected tools of the caller's ever
        /// held from that daemon's tool, as its list item carries it.
        index: u64,
        /// Its name, as its create gave it, if it gave one. Absent when
        /// it has none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A dependency tool: running from a template an agent's program
    /// declared, transient or a service.
    Dependency {
        /// The dependency template it runs from, by id — the hash of
        /// the template's canonical bytes.
        template: String,
        /// Whose it is: see [`Owner`].
        owner: Owner,
    },
}
