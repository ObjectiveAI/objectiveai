//! The agent that made an agent.

use serde::{Deserialize, Serialize};

/// The agent that made another: named three ways, so that it is the
/// same agent whether or not it still exists and whatever name a
/// later agent has taken. The template it was made from, its
/// [`count`](Self::count) among the agents ever made from that
/// template, which together name it once and for all, and its name
/// as its create gave it, which names it now.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Creator {
    /// The template the creator was made from, by id.
    pub template: String,
    /// The creator's number among all agents of the caller's ever
    /// made from that template: see [`Agent::count`](super::Agent::count).
    pub count: u64,
    /// The creator's name, as its create gave it.
    pub name: String,
}
