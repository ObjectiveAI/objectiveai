//! What is an agent template's own: its type, and the daemon's
//! tools its agents hold.

use serde::{Deserialize, Serialize};

use crate::daemon::builtin::Builtin;
use super::AgentType;

/// The front of an agent template, flattened into its JSON: the
/// `type`, always `"agent"`, and the daemon's
/// [`builtin`](crate::daemon::builtin) tools every agent made from
/// the template holds. `{"type":"agent"}` for a template whose
/// agents hold none, which hashes as it did before there were any.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentKind {
    /// `"agent"`. See [`AgentType`].
    pub r#type: AgentType,
    /// The daemon's tools the agents hold: see [`Builtin`]. Absent
    /// when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin: Option<Builtin>,
}

/// An agent template: the daemon's one
/// [`Template`](crate::daemon::template::Template), led by
/// [`AgentKind`].
pub type Template = crate::daemon::template::Template<AgentKind>;
