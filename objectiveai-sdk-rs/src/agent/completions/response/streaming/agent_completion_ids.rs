//! [`AgentCompletionIds`] — borrowed iterator over the agent-completion
//! `response_id`s carried by any streaming chunk type.
//!
//! Used by the API server's WS handler to scope per-stream
//! `agent_completion_notify` validation: every chunk passing through
//! the send side extends a per-connection HashSet with its
//! `agent_completion_ids()`; incoming notifies are accepted only if
//! their `response_id` is in that set.
//!
//! Composite chunks delegate to their children. **Never collect** —
//! the iterators borrow into `self` with zero allocation.

/// Trait implemented by every streaming chunk type that can carry
/// agent-completion `response_id`s. Leaves return a singleton iterator;
/// composites flat-map over their children.
pub trait AgentCompletionIds {
    /// Yields every agent-completion `response_id` reachable from this
    /// chunk. Borrows into `self`; never allocates.
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str>;
}

impl AgentCompletionIds for super::AgentCompletionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id.as_str())
    }
}
