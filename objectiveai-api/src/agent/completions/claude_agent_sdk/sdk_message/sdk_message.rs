use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SDKMessage {
    AssistantMessage(super::SDKAssistantMessage),
    UserMessage(super::SDKUserMessage),
    UserMessageReplay(super::SDKUserMessageReplay),
    ResultMessage(super::SDKResultMessage),
    SystemMessage(super::SDKSystemMessage),
    PartialAssistantMessage(super::SDKPartialAssistantMessage),
    CompactBoundaryMessage(super::SDKCompactBoundaryMessage),
    StatusMessage(super::SDKStatusMessage),
    HookStartedMessage(super::SDKHookStartedMessage),
    HookProgressMessage(super::SDKHookProgressMessage),
    HookResponseMessage(super::SDKHookResponseMessage),
    ToolProgressMessage(super::SDKToolProgressMessage),
    AuthStatusMessage(super::SDKAuthStatusMessage),
    TaskNotificationMessage(super::SDKTaskNotificationMessage),
    TaskStartedMessage(super::SDKTaskStartedMessage),
    FilesPersistedEvent(super::SDKFilesPersistedEvent),
    ToolUseSummaryMessage(super::SDKToolUseSummaryMessage),
    RateLimitEvent(super::SDKRateLimitEvent),
}

impl SDKMessage {
    /// Returns the session ID if this message variant carries one.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::PartialAssistantMessage(msg) => Some(&msg.session_id),
            Self::ResultMessage(msg) => Some(msg.session_id()),
            Self::UserMessage(msg) => msg.session_id.as_deref(),
            Self::AssistantMessage(msg) => Some(&msg.session_id),
            _ => None,
        }
    }

    /// Transforms this upstream SDK message into a downstream
    /// [`AgentCompletionChunk`].
    ///
    /// Returns `Some(Ok(chunk))` for messages that produce streaming data,
    /// `Some(Err(Error::RateLimit))` for rate limit events, and `None` for
    /// messages that should be ignored.
    #[allow(clippy::too_many_arguments)]
    pub fn into_downstream(
        self,
        id: String,
        created: u64,
        assistant_index: u64,
        is_byok: bool,
        cost_multiplier: rust_decimal::Decimal,
        // per-1-SECOND duration rate + this upstream's create→finish
        // elapsed; used only by the terminal ResultMessage.
        duration_cost: rust_decimal::Decimal,
        elapsed_ms: u64,
        upstream: objectiveai_sdk::agent::Upstream,
        agent_instance_hierarchy: String,
        agent_id: String,
        agent_full_id: String,
        agent_remote: Option<objectiveai_sdk::RemotePath>,
    ) -> Option<
        Result<
            objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk,
            super::super::Error,
        >,
    > {
        match self {
            Self::PartialAssistantMessage(msg) => {
                msg.into_downstream(
                    id, created, assistant_index, upstream,
                    agent_instance_hierarchy, agent_id, agent_full_id, agent_remote,
                ).map(Ok)
            }
            Self::UserMessage(msg) => {
                msg.into_downstream(
                    id, created, assistant_index, upstream,
                    agent_instance_hierarchy, agent_id, agent_full_id, agent_remote,
                ).map(Ok)
            }
            Self::ResultMessage(msg) => {
                Some(Ok(msg.into_downstream(
                    id, created, assistant_index, is_byok, cost_multiplier,
                    duration_cost, elapsed_ms, upstream,
                    agent_instance_hierarchy, agent_id, agent_full_id, agent_remote,
                )))
            }
            // Rate-limit events come in two shapes:
            //   type="rate_limit_event" with rate_limit_info.status — a
            //     status report. claude emits status="allowed" on every
            //     successful call (informational); we only treat it as an
            //     error when status is "rejected".
            //   type="rate_limit" — terminal rate-limit signal (no info).
            Self::RateLimitEvent(evt) => {
                use super::sdk_rate_limit_event::{RateLimitEventType, RateLimitStatus};
                let rejected = evt
                    .rate_limit_info
                    .and_then(|i| i.status)
                    .map(|s| matches!(s, RateLimitStatus::Rejected))
                    .unwrap_or(false);
                let terminal = matches!(evt.r#type, RateLimitEventType::RateLimit);
                if rejected || terminal {
                    Some(Err(super::super::Error::RateLimit))
                } else {
                    None
                }
            }
            // All other variants are ignored.
            Self::AssistantMessage(_)
            | Self::UserMessageReplay(_)
            | Self::SystemMessage(_)
            | Self::CompactBoundaryMessage(_)
            | Self::StatusMessage(_)
            | Self::HookStartedMessage(_)
            | Self::HookProgressMessage(_)
            | Self::HookResponseMessage(_)
            | Self::ToolProgressMessage(_)
            | Self::AuthStatusMessage(_)
            | Self::TaskNotificationMessage(_)
            | Self::TaskStartedMessage(_)
            | Self::FilesPersistedEvent(_)
            | Self::ToolUseSummaryMessage(_) => None,
        }
    }
}
