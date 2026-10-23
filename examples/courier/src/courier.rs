//! The MCP server: the courier's state, and the four tools served.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult, MetaObject, PaginatedRequestParams, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::caller::{Participant, caller};
use crate::daemon::Daemon;
use crate::failure::Failure;
use crate::register;
use crate::store::Store;
use crate::tools;

/// What every MCP session shares: the proxy's client, the database
/// pool and the daemon connection, each made on first use.
#[derive(Default)]
pub struct State {
    /// The proxy, from the inside.
    pub client: Client,
    /// The mail.
    pub store: Store,
    /// The daemon, for looking agents up and notifying them.
    pub daemon: Daemon,
}

/// The server one MCP session is served by: every session the proxy
/// opens is a clone of one courier, so the state is the container's.
#[derive(Clone, Default)]
pub struct Courier {
    state: Arc<State>,
}

/// What a tool call has to work with: the shared state, the template
/// the mail is kept under, and the caller the daemon attested.
pub struct Call<'a> {
    /// The shared state.
    pub state: &'a State,
    /// The template the courier was made from.
    pub template: &'a str,
    /// The agent calling.
    pub caller: Participant,
}

impl Courier {
    /// A courier, holding nothing yet.
    pub fn new() -> Courier {
        Courier::default()
    }
}

/// The instructions an agent's MCP client is handed on connecting.
const INSTRUCTIONS: &str = "An inbox between agents. See whom you can write to with `list_agents`. Open a conversation with another agent with `create_conversation`, answer one with `reply`. \
Nothing is put into your conversation but a notice, `<courier>You have a new message</courier>`, when a message arrives: \
list your conversations with unread messages with `list_conversations`, then read one with `list_messages`. Both show unread only unless \
`include_read` is true.";

impl ServerHandler for Courier {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("courier", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools::all()))
    }

    /// Every call: the registration, then the attested caller, then
    /// the tool by name. A tool's own failure is its error result, for
    /// the agent to read; only a tool the courier does not have is a
    /// protocol error.
    async fn call_tool(&self, request: CallToolRequestParams, context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        if !tools::NAMES.contains(&request.name.as_ref()) {
            return Err(ErrorData::invalid_params(format!("the courier has no tool named {}", request.name), None));
        }
        let outcome = async {
            let template = register::registered().ok_or(Failure::Unregistered)?.template.as_str();
            // rmcp moves a request's `_meta` into the context before the
            // handler runs; the params' own is read only when the
            // context holds no attestation.
            let attested = |meta: &MetaObject| meta.0.contains_key(diverge_sdk::shared::mcp::KEY);
            let meta = match request.meta.as_ref() {
                Some(meta) if !attested(&context.meta.0) && attested(&meta.0) => &meta.0,
                _ => &context.meta.0,
            };
            let call = Call { state: &self.state, template, caller: caller(meta)? };
            tools::call(&request.name, call, request.arguments.unwrap_or_default()).await
        }
        .await;
        Ok(CallToolResponse::from(match outcome {
            Ok(value) => CallToolResult::structured(value),
            Err(failure) => CallToolResult::error(vec![rmcp::model::ContentBlock::text(failure.to_string())]),
        }))
    }
}
