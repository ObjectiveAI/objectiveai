//! The container's tool calls outward: the served tools, merged.

use std::sync::Arc;

use diverge_sdk::provider::client::McpServer;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData, ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};

use super::Answerer;
use crate::containers::mcp::{self, Context, Notifications};

impl Answerer {
    /// The merge's context for this run.
    fn context(&self) -> Context {
        Context {
            daemon: Arc::clone(&self.daemon),
            user: self.key,
            caller: self.caller.clone(),
            root: self.root.clone(),
            chain: self.chain.clone(),
            served: Arc::clone(&self.served),
        }
    }
}

/// The five exchanges over the run's served tools, each a use of the
/// container.
impl McpServer for Answerer {
    type Notifications = Notifications;

    async fn list_tools(&self, params: Option<PaginatedRequestParams>) -> Result<ListToolsResult, ErrorData> {
        self.touch();
        mcp::list_tools(&self.context(), params).await
    }

    async fn list_resources(&self, params: Option<PaginatedRequestParams>) -> Result<ListResourcesResult, ErrorData> {
        self.touch();
        mcp::list_resources(&self.context(), params).await
    }

    async fn call_tool(&self, params: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
        self.touch();
        mcp::call_tool(&self.context(), params).await
    }

    async fn read_resource(&self, params: ReadResourceRequestParams) -> Result<ReadResourceResult, ErrorData> {
        self.touch();
        mcp::read_resource(&self.context(), params).await
    }

    async fn notifications(&self) -> Self::Notifications {
        mcp::notifications(&self.context()).await
    }
}
