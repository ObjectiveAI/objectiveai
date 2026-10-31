//! What an agent is doing now, as a judgement reads it.

/// An agent's state now.
///
/// The rules:
///
/// 1. `active`: a loop runs in it — what a list reports as `active`,
///    and what a filter's or a grant's `active` reads.
/// 2. `deployed`: its container is up — what a list reports as
///    `deployed`, and what a filter's or a grant's `deployed` reads.
/// 3. `busy`: a loop runs in it or an exchange is in flight for it —
///    what refuses a delete; no list reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgentState {
    /// Rule 1.
    pub active: bool,
    /// Rule 2.
    pub deployed: bool,
    /// Rule 3.
    pub busy: bool,
}
