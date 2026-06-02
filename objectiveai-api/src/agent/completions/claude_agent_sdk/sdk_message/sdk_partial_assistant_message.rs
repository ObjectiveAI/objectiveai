use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SDKPartialAssistantMessageType {
    StreamEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SDKPartialAssistantMessage {
    pub r#type: SDKPartialAssistantMessageType,
    pub event: super::super::beta_raw_message_stream_event::BetaRawMessageStreamEvent,
    pub parent_tool_use_id: Option<String>,
    pub uuid: String,
    pub session_id: String,
}

impl SDKPartialAssistantMessage {
    /// Transforms this upstream partial assistant message into a downstream
    /// [`AgentCompletionChunk`], or `None` if the inner event should be ignored.
    #[allow(clippy::too_many_arguments)]
    pub fn into_downstream(
        self,
        id: String,
        created: u64,
        assistant_index: u64,
        upstream: objectiveai_sdk::agent::Upstream,
        agent_instance_hierarchy: String,
        agent_id: String,
        agent_full_id: String,
        agent_remote: Option<objectiveai_sdk::RemotePath>,
    ) -> Option<objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk> {
        self.event.into_downstream(
            id, created, assistant_index, self.session_id, upstream,
            agent_instance_hierarchy, agent_id, agent_full_id, agent_remote,
        )
    }
}
