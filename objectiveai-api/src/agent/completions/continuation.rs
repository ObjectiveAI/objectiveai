use objectiveai_sdk::mcp;

#[derive(Debug, Clone)]
pub enum Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK> {
    Openrouter {
        items: Vec<ContinuationItem<OPENROUTER>>,
        mcp_connection: Option<mcp::Connection>,
        /// Full slash-separated lineage of the agent this continuation
        /// belongs to (e.g. `A/B/<11-char-rand-b62><base62-created>`). Minted on
        /// the agent's first spawn and preserved verbatim across every
        /// continuation round — server-side internal retries AND
        /// client-side resume — so the agent's identity stays stable
        /// regardless of who resumes the conversation. The trailing
        /// segment is the local `id` used as `AgentCompletionChunk.id`.
        agent_id: String,
    },
    ClaudeAgentSdk {
        items: Vec<ContinuationItem<CLAUDEAGENTSDK>>,
        mcp_connection: Option<mcp::Connection>,
        agent_id: String,
    },
    CodexSdk {
        items: Vec<ContinuationItem<CODEXSDK>>,
        mcp_connection: Option<mcp::Connection>,
        agent_id: String,
    },
    Mock {
        items: Vec<ContinuationItem<MOCK>>,
        mcp_connection: Option<mcp::Connection>,
        agent_id: String,
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

    /// Full slash-separated lineage of the agent this continuation
    /// belongs to. Stable across every continuation round. See the
    /// per-variant field doc.
    pub fn agent_id(&self) -> &str {
        match self {
            Self::Openrouter { agent_id, .. }
            | Self::ClaudeAgentSdk { agent_id, .. }
            | Self::CodexSdk { agent_id, .. }
            | Self::Mock { agent_id, .. } => agent_id.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ContinuationItem<STATE> {
    State(STATE),
    ToolMessage(objectiveai_sdk::agent::completions::message::ToolMessage),
    UserMessage(objectiveai_sdk::agent::completions::message::UserMessage),
}
