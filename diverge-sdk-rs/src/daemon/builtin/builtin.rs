//! Which of the daemon's tools an agent holds.

use serde::{Deserialize, Serialize};

use super::{Agents, AgentsCreate, AgentsDelete, AgentsEdit, Tags, TemplatesDelete};

/// The daemon's tools an agent made from the template holds, one
/// member each. A member present gives the agent that tool, as the
/// member says; a member absent withholds it. Part of the template,
/// and so of its hash; every member absent is the same as no
/// `builtin` at all, and hashes the same.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Builtin {
    /// The tool over agents as they are: see [`Agents`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<Agents>,
    /// The tool that makes agents: see [`AgentsCreate`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_create: Option<AgentsCreate>,
    /// The tool that deletes agents: see [`AgentsDelete`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_delete: Option<AgentsDelete>,
    /// The tool that edits agents: see [`AgentsEdit`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents_edit: Option<AgentsEdit>,
    /// The tool that makes agent templates: `true`, the agent has
    /// it; `false`, or absent, it does not.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub templates_create: bool,
    /// The tool that deletes agent templates: see
    /// [`TemplatesDelete`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub templates_delete: Option<TemplatesDelete>,
    /// The tools that tag and untag, and how far they reach: see
    /// [`Tags`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Tags>,
}
