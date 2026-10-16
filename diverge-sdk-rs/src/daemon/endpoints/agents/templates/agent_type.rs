//! What an agent template says of itself beside what every template
//! says: that it is one.

use serde::{Deserialize, Serialize};

/// `"type":"agent"`, and nothing beside it: what tells an agent
/// template from a tool template with the same image, limits, mounts,
/// arguments and secrets. Flat among the template's members: a template
/// whose `type` is anything else does not decode as an agent
/// template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename = "agent")]
pub struct AgentType {}

/// An agent template: the daemon's one
/// [`Template`](crate::daemon::template::Template), typed
/// [`AgentType`].
pub type Template = crate::daemon::template::Template<AgentType>;
