use objectiveai_sdk::mcp;

#[derive(Debug, Clone)]
pub enum Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK> {
    Openrouter {
        items: Vec<ContinuationItem<OPENROUTER>>,
        mcp_connection: Option<mcp::Connection>,
        /// Per-(parent_agent_id, this_agent_id) index allocated when this
        /// agent run was first opened. Reused across continuation turns
        /// so the composite X-OBJECTIVEAI-AGENT-ID stays stable for the
        /// life of a single agent invocation.
        agent_index: u64,
    },
    ClaudeAgentSdk {
        items: Vec<ContinuationItem<CLAUDEAGENTSDK>>,
        mcp_connection: Option<mcp::Connection>,
        agent_index: u64,
    },
    CodexSdk {
        items: Vec<ContinuationItem<CODEXSDK>>,
        mcp_connection: Option<mcp::Connection>,
        agent_index: u64,
    },
    Mock {
        items: Vec<ContinuationItem<MOCK>>,
        mcp_connection: Option<mcp::Connection>,
        agent_index: u64,
    },
}

impl<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>
    Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>
{
    pub fn push_user_message(&mut self, message: objectiveai_sdk::agent::completions::message::UserMessage) {
        match self {
            Self::Openrouter { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::ClaudeAgentSdk { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::CodexSdk { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::Mock { items, .. } => items.push(ContinuationItem::UserMessage(message)),
        }
    }

    pub fn push_tool_message(&mut self, message: objectiveai_sdk::agent::completions::message::ToolMessage) {
        match self {
            Self::Openrouter { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::ClaudeAgentSdk { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::CodexSdk { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::Mock { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
        }
    }

    pub fn upstream(&self) -> objectiveai_sdk::agent::Upstream {
        match self {
            Self::Openrouter { .. } => objectiveai_sdk::agent::Upstream::Openrouter,
            Self::ClaudeAgentSdk { .. } => objectiveai_sdk::agent::Upstream::ClaudeAgentSdk,
            Self::CodexSdk { .. } => objectiveai_sdk::agent::Upstream::CodexSdk,
            Self::Mock { .. } => objectiveai_sdk::agent::Upstream::Mock,
        }
    }

    /// The single MCP proxy connection for this agent (or `None` if the
    /// agent had no MCP servers and no invention tools).
    pub fn mcp_connection(&self) -> Option<&mcp::Connection> {
        match self {
            Self::Openrouter { mcp_connection, .. }
            | Self::ClaudeAgentSdk { mcp_connection, .. }
            | Self::CodexSdk { mcp_connection, .. }
            | Self::Mock { mcp_connection, .. } => mcp_connection.as_ref(),
        }
    }

    /// Per-`(parent_agent_id, this_agent_id)` index assigned when this
    /// run started. See the corresponding doc on the enum variants.
    pub fn agent_index(&self) -> u64 {
        match self {
            Self::Openrouter { agent_index, .. }
            | Self::ClaudeAgentSdk { agent_index, .. }
            | Self::CodexSdk { agent_index, .. }
            | Self::Mock { agent_index, .. } => *agent_index,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ContinuationItem<STATE> {
    State(STATE),
    ToolMessage(objectiveai_sdk::agent::completions::message::ToolMessage),
    UserMessage(objectiveai_sdk::agent::completions::message::UserMessage),
}
