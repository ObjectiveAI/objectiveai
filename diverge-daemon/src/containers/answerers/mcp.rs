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
            served: Arc::clone(&self.served),
        }
    }
}

/// The five exchanges over the run's served tools, each a use of the
/// container and, for the four that are answered, in flight for the
/// agent from the ask to the answer — however the answer comes,
/// since the guard counts out when the future ends.
impl McpServer for Answerer {
    type Notifications = Notifications;

    async fn list_tools(&self, params: Option<PaginatedRequestParams>) -> Result<ListToolsResult, ErrorData> {
        let _flying = self.inflight.enter();
        self.touch();
        mcp::list_tools(&self.context(), params).await
    }

    async fn list_resources(&self, params: Option<PaginatedRequestParams>) -> Result<ListResourcesResult, ErrorData> {
        let _flying = self.inflight.enter();
        self.touch();
        mcp::list_resources(&self.context(), params).await
    }

    async fn call_tool(&self, params: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
        let _flying = self.inflight.enter();
        self.touch();
        mcp::call_tool(&self.context(), params).await
    }

    async fn read_resource(&self, params: ReadResourceRequestParams) -> Result<ReadResourceResult, ErrorData> {
        let _flying = self.inflight.enter();
        self.touch();
        mcp::read_resource(&self.context(), params).await
    }

    async fn notifications(&self) -> Self::Notifications {
        mcp::notifications(&self.context()).await
    }
}
