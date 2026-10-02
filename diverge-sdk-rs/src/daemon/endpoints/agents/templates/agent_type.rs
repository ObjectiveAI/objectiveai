//! The one value an agent template's `type` takes.

use serde::{Deserialize, Serialize};

/// `"agent"`, and nothing else: the `type` of an agent template,
/// which is what tells it from a tool template with the same image,
/// limits, mounts and arguments. A template whose `type` is anything
/// else does not decode as an agent template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// The only value.
    #[default]
    Agent,
}

/// An agent template: the daemon's one
/// [`Template`](crate::daemon::template::Template), typed
/// [`AgentType`].
pub type Template = crate::daemon::template::Template<AgentType>;
