use objectiveai::mcp;

#[derive(Debug, Clone)]
pub enum Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK> {
    Openrouter {
        items: Vec<ContinuationItem<OPENROUTER>>,
        mcp_connection: Option<mcp::Connection>,
    },
    ClaudeAgentSdk {
        items: Vec<ContinuationItem<CLAUDEAGENTSDK>>,
        mcp_connection: Option<mcp::Connection>,
    },
    CodexSdk {
        items: Vec<ContinuationItem<CODEXSDK>>,
        mcp_connection: Option<mcp::Connection>,
    },
    Mock {
        items: Vec<ContinuationItem<MOCK>>,
        mcp_connection: Option<mcp::Connection>,
    },
}

impl<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>
    Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>
{
    pub fn push_user_message(&mut self, message: objectiveai::agent::completions::message::UserMessage) {
        match self {
            Self::Openrouter { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::ClaudeAgentSdk { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::CodexSdk { items, .. } => items.push(ContinuationItem::UserMessage(message)),
            Self::Mock { items, .. } => items.push(ContinuationItem::UserMessage(message)),
        }
    }

    pub fn push_tool_message(&mut self, message: objectiveai::agent::completions::message::ToolMessage) {
        match self {
            Self::Openrouter { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::ClaudeAgentSdk { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::CodexSdk { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
            Self::Mock { items, .. } => items.push(ContinuationItem::ToolMessage(message)),
        }
    }

    pub fn upstream(&self) -> objectiveai::agent::Upstream {
        match self {
            Self::Openrouter { .. } => objectiveai::agent::Upstream::Openrouter,
            Self::ClaudeAgentSdk { .. } => objectiveai::agent::Upstream::ClaudeAgentSdk,
            Self::CodexSdk { .. } => objectiveai::agent::Upstream::CodexSdk,
            Self::Mock { .. } => objectiveai::agent::Upstream::Mock,
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
}

#[derive(Debug, Clone)]
pub enum ContinuationItem<STATE> {
    State(STATE),
    ToolMessage(objectiveai::agent::completions::message::ToolMessage),
    UserMessage(objectiveai::agent::completions::message::UserMessage),
}
