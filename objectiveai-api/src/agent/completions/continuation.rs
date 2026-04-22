use std::collections::HashSet;
use std::sync::Arc;
use crate::mcp;

#[derive(Debug, Clone)]
pub enum Continuation<OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK> {
    Openrouter {
        items: Vec<ContinuationItem<OPENROUTER>>,
        mcp_connections: Arc<Vec<Arc<mcp::Connection>>>,
    },
    ClaudeAgentSdk {
        items: Vec<ContinuationItem<CLAUDEAGENTSDK>>,
        mcp_connections: Arc<Vec<Arc<mcp::Connection>>>,
    },
    ClaudeCode {
        items: Vec<ContinuationItem<CLAUDECODE>>,
        mcp_connections: Arc<Vec<Arc<mcp::Connection>>>,
    },
    Mock {
        items: Vec<ContinuationItem<MOCK>>,
        mcp_connections: Arc<Vec<Arc<mcp::Connection>>>,
    },
}

impl<OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK> Continuation<OPENROUTER, CLAUDEAGENTSDK, CLAUDECODE, MOCK> {
    pub fn push_user_message(&mut self, message: objectiveai::agent::completions::message::UserMessage) {
        match self {
            Self::Openrouter { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::ClaudeAgentSdk { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::ClaudeCode { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::Mock { items, .. } => items.push(ContinuationItem::UserMessage(message)),
        }
    }

    pub fn push_tool_message(&mut self, message: objectiveai::agent::completions::message::ToolMessage) {
        match self {
            Self::Openrouter { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::ClaudeAgentSdk { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::ClaudeCode { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::Mock { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
        }
    }

    pub fn upstream(&self) -> objectiveai::agent::Upstream {
        match self {
            Self::Openrouter { .. } => objectiveai::agent::Upstream::Openrouter,
            Self::ClaudeAgentSdk { .. } => objectiveai::agent::Upstream::ClaudeAgentSdk,
            Self::ClaudeCode { .. } => objectiveai::agent::Upstream::ClaudeCode,
            Self::Mock { .. } => objectiveai::agent::Upstream::Mock,
        }
    }

    pub fn mcp_connections(&self) -> &Arc<Vec<Arc<mcp::Connection>>> {
        match self {
            Self::Openrouter { mcp_connections, .. }
            | Self::ClaudeAgentSdk { mcp_connections, .. }
            | Self::ClaudeCode { mcp_connections, .. }
            | Self::Mock { mcp_connections, .. } => mcp_connections,
        }
    }

    pub fn mcp_urls(&self) -> HashSet<String> {
        self.mcp_connections().iter().map(|c| c.url.clone()).collect()
    }
}

#[derive(Debug, Clone)]
pub enum ContinuationItem<STATE> {
    State(STATE),
    ToolMessage(objectiveai::agent::completions::message::ToolMessage),
    UserMessage(objectiveai::agent::completions::message::UserMessage),
}
