//! What an agent is doing now, as a judgement reads it.

/// An agent's state now.
///
/// The rules:
///
/// 1. `active`: a loop runs in it, or an MCP exchange is in flight
///    for it — what a list reports as `active`, what a filter's or a
///    grant's `active` reads, and what refuses a delete.
/// 2. `deployed`: its container is up — what a list reports as
///    `deployed`, and what a filter's or a grant's `deployed` reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgentState {
    /// Rule 1.
    pub active: bool,
    /// Rule 2.
    pub deployed: bool,
}
