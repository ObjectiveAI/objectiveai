//! One agent, as a grant lists it.

use serde::{Deserialize, Serialize};

/// One agent a grant reaches, by what names it once and for all on the
/// daemon that holds it: the template it was made from and its number
/// among every agent that daemon ever made from that template. The
/// pair is given to nothing else ever, so a listed agent that is
/// deleted is reached by nothing after. No name is carried: a name is
/// optional and may be given again, and a grant must not come to reach
/// another agent by one being renamed. No other member decodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Key {
    /// The template the agent was made from, by id.
    pub template: String,
    /// The agent's number among all agents the daemon ever made from
    /// that template.
    pub index: u64,
}
