//! `agents read subscribe` — async handler stub.

use crate::cli::command::IntoCommand;
use crate::filesystem::db::MessageKind;

pub struct Request {
    pub agent_instance_hierarchy: String,
    pub kind: Option<MessageKind>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "read".to_string(),
            "subscribe".to_string(),
            self.agent_instance_hierarchy.clone(),
        ];
        if let Some(kind) = &self.kind {
            argv.push("--kind".to_string());
            argv.push(message_kind_flag(kind).to_string());
        }
        argv
    }
}

fn message_kind_flag(kind: &MessageKind) -> &'static str {
    // Wire form matches clap's `value_enum` rename_all = "kebab-case" default.
    match kind {
        MessageKind::AgentCompletionRequest => "agent-completion-request",
        MessageKind::AgentCompletionResponse => "agent-completion-response",
        MessageKind::AgentCompletionMessage => "agent-completion-message",
        MessageKind::AssistantResponse => "assistant-response",
        MessageKind::ContinuationToken => "continuation-token",
        MessageKind::Sweep => "sweep",
    }
}

pub enum ResponseItem {
    Items {
        agent_id: String,
        items: Vec<crate::filesystem::logs::queue::QueueItem>,
    },
    Inactive {
        agent_id: String,
    },
}
