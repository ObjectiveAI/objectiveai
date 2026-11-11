//! The MCP server: the board's state, and the eight tools served.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult, MetaObject, PaginatedRequestParams, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::caller::{Participant, caller};
use crate::store::Store;
use crate::tools;

/// What every MCP session shares: the proxy's client and the database
/// pool, made on first use.
#[derive(Default)]
pub struct State {
    /// The proxy, from the inside.
    pub client: Client,
    /// The board.
    pub store: Store,
}

/// The server one MCP session is served by: every session the proxy
/// opens is a clone of one taskboard, so the state is the container's.
#[derive(Clone, Default)]
pub struct Taskboard {
    state: Arc<State>,
}

/// What a tool call has to work with: the shared state and the caller
/// the daemon attested.
pub struct Call<'a> {
    /// The shared state.
    pub state: &'a State,
    /// The agent calling.
    pub caller: Participant,
}

impl Taskboard {
    /// A taskboard, holding nothing yet.
    pub fn new() -> Taskboard {
        Taskboard::default()
    }
}

/// The instructions an agent's MCP client is handed on connecting.
const INSTRUCTIONS: &str = "A board of tasks shared with other agents. Post a task with `create_task` and see the board with `list_tasks`. \
Take an open task with `claim_task`; no two agents can hold the same one. Finish it with `complete_task`, or give it back with \
`release_task`; only the agent holding a task can do either. Discuss a task with `create_comment` and `list_comments`. Delete a task \
you posted with `delete_task`. Nothing is sent to you when the board changes: look at it when you choose.";

impl ServerHandler for Taskboard {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("taskboard", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools::all()))
    }

    /// Every call: the attested caller, then the tool by name. A
    /// tool's own failure is its error result, for the agent to read;
    /// only a tool the board does not have is a protocol error.
    async fn call_tool(&self, request: CallToolRequestParams, context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        if !tools::NAMES.contains(&request.name.as_ref()) {
            return Err(ErrorData::invalid_params(format!("the taskboard has no tool named {}", request.name), None));
        }
        let outcome = async {
            // rmcp moves a request's `_meta` into the context before the
            // handler runs; the params' own is read only when the
            // context holds no attestation.
            let attested = |meta: &MetaObject| meta.0.contains_key(diverge_sdk::shared::mcp::KEY);
            let meta = match request.meta.as_ref() {
                Some(meta) if !attested(&context.meta.0) && attested(&meta.0) => &meta.0,
                _ => &context.meta.0,
            };
            let call = Call { state: &self.state, caller: caller(meta)? };
            tools::call(&request.name, call, request.arguments.unwrap_or_default()).await
        }
        .await;
        Ok(CallToolResponse::from(match outcome {
            Ok(value) => CallToolResult::structured(value),
            Err(failure) => CallToolResult::error(vec![rmcp::model::ContentBlock::text(failure.to_string())]),
        }))
    }
}
