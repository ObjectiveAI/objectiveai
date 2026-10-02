//! An agent as a creator.

use serde::{Deserialize, Serialize};

/// An agent of the client's that made something: named once and for
/// all by its template and its count, and by its name as it was
/// called.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Agent {
    /// The template the agent was made from, by id.
    pub template: String,
    /// The agent's number among all agents of the client's ever made
    /// from that template: the
    /// [`count`](crate::daemon::endpoints::agents::list::server::response::Agent::count)
    /// its list item carries. The template and the count name the
    /// agent once and for all.
    pub count: u64,
    /// The agent's name, as its create gave it.
    pub name: String,
}
