//! The MCP server: the tool's state, and the three tools served.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::store::Store;
use crate::tools;

/// What every MCP session shares: the proxy's client and the database
/// pool, made on first use.
#[derive(Default)]
pub struct State {
    /// The proxy, from the inside.
    pub client: Client,
    /// The notes.
    pub store: Store,
}

/// The server one MCP session is served by: every session the proxy
/// opens is a clone of one memory, so the state is the container's.
#[derive(Clone, Default)]
pub struct Memory {
    state: Arc<State>,
}

impl Memory {
    /// A memory, holding nothing yet.
    pub fn new() -> Memory {
        Memory::default()
    }
}

/// The instructions an agent's MCP client is handed on connecting.
const INSTRUCTIONS: &str = "A notebook that outlasts your conversation. Keep a note with `remember`, optionally tagged. Find notes again with \
`recall`: by a search of their text, by tags, or both; with neither, the newest. Delete one with `forget`.";

impl ServerHandler for Memory {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("memory", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools::all()))
    }

    /// Every call: the tool by name. A tool's own failure is its error
    /// result, for the agent to read; only a tool the memory does not
    /// have is a protocol error. The caller is never asked for: the
    /// notebook is the database's, whoever calls.
    async fn call_tool(&self, request: CallToolRequestParams, _context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        if !tools::NAMES.contains(&request.name.as_ref()) {
            return Err(ErrorData::invalid_params(format!("the memory tool has no tool named {}", request.name), None));
        }
        let outcome = tools::call(&request.name, &self.state, request.arguments.unwrap_or_default()).await;
        Ok(CallToolResponse::from(match outcome {
            Ok(value) => CallToolResult::structured(value),
            Err(failure) => CallToolResult::error(vec![rmcp::model::ContentBlock::text(failure.to_string())]),
        }))
    }
}
