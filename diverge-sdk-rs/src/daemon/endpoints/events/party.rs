//! One end of an edge: who sent, or who received.

use serde::{Deserialize, Serialize};

/// One party to an [`Event`](super::Event), named once and for all by
/// what never changes and never by its name: an account by its
/// identity, and an agent, a standalone tool or a dependency tool by
/// its template and its index among those made from that template.
/// Internally tagged on the wire by `kind`, snake case:
/// `{"kind":"account","issuer":…,"subject":…}`,
/// `{"kind":"agent","template":…,"index":…}`,
/// `{"kind":"standalone_tool","template":…,"index":…}` or
/// `{"kind":"dependency_tool","template":…,"index":…}`.
///
/// A tool connected from another daemon is never a party: it stands
/// as the party that connected it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Party {
    /// An account, or an administrator, over an endpoint: its
    /// identity, see [`accounts`](crate::daemon::endpoints::accounts).
    Account {
        /// The identity's issuer.
        issuer: String,
        /// The identity's subject.
        subject: String,
    },
    /// An agent: see [`agents`](crate::daemon::endpoints::agents).
    Agent {
        /// The template it was made from, by id.
        template: String,
        /// Its number among all agents ever made from that template,
        /// as its list item carries it.
        index: u64,
    },
    /// A tool made from a template: see
    /// [`standalone_tools`](crate::daemon::endpoints::standalone_tools).
    StandaloneTool {
        /// The template it was made from, by id.
        template: String,
        /// Its number among all tools ever made from that template,
        /// as its list item's origin carries it.
        index: u64,
    },
    /// A dependency tool: see
    /// [`dependency_tools`](crate::daemon::endpoints::dependency_tools).
    DependencyTool {
        /// The dependency template it runs from, by id.
        template: String,
        /// Its number among all dependency tools ever made from that
        /// template, as its list item carries it.
        index: u64,
    },
}
