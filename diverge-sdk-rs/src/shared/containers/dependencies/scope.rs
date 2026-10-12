//! How far a service is shared.

use serde::{Deserialize, Serialize};

/// What a service tool is one of: one per parent agent, one per
/// parent agent template, or one for every agent that declares it,
/// whatever template they are made from. The scope names the service
/// — the same template declared under the same scope key is the same
/// service, joined and not made again — and names its database scope
/// with it, so what a service holds is shared exactly as far as the
/// service is. Snake case on the wire: `"per_agent_instance"`,
/// `"per_agent_template"`, `"global"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// One service per parent agent: the service deployed for one
    /// agent is its own, with a database of its own, and two agents'
    /// services of this template never see each other's. The scope a
    /// transient tool has without saying so.
    PerAgentInstance,
    /// One service per parent agent template: every agent made from
    /// the same agent template that declares this template is
    /// attached to one service, which holds one database, so what one
    /// writes the others read.
    PerAgentTemplate,
    /// One service for the template, full stop: every agent that
    /// declares it, from whatever agent template, is attached to the
    /// one service, and its database is one for all of them.
    Global,
}
