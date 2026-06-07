/// Handle to a notification whose log file has been written and
/// whose per-agent DB index has been reserved by
/// [`super::messages::Queue::write_notification`].
///
/// The cli-stream writer task queues these locally and passes them
/// back into [`super::messages::Queue::insert_notification`] when the
/// next tool response for the same agent comes in — or at stream end
/// via the writer's `finalize`.
#[derive(Debug, Clone)]
pub struct PendingNotification {
    /// Lineage-stamped agent id (`{caller}/{response_id}` or
    /// `{response_id}` at the root). Used as the per-agent
    /// reservation namespace and as the agent column on the row.
    pub agent_instance_hierarchy: String,
    /// The bare agent-completion response id the notification
    /// targets. Stored explicitly so the reader doesn't have to
    /// parse it out of `agent_instance_hierarchy`.
    pub response_id: String,
    pub index: u64,
    pub path: String,
    pub timestamp: u64,
}
