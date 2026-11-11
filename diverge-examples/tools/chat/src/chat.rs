//! The chat's state, shared by every MCP session and every page, and
//! the MCP server.

use std::collections::HashMap;
use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult, MetaObject, PaginatedRequestParams, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use tokio::sync::{Mutex, broadcast};

use crate::caller::{Participant, agent};
use crate::daemon::Daemon;
use crate::store::{Message, Store};
use crate::tools;

/// How many messages an open page may fall behind before it misses
/// some; a page that does is told, and reloads the history.
const ROOM: usize = 256;

/// What every MCP session and every page share: the proxy's client,
/// the database, the daemon, and the open pages of each conversation.
#[derive(Default)]
pub struct State {
    /// The proxy, from the inside.
    pub client: Client,
    /// The messages.
    pub store: Store,
    /// The daemon.
    pub daemon: Daemon,
    /// Each conversation's fan-out to its open pages, by the agent's
    /// template and index, made when first asked for.
    rooms: Mutex<HashMap<(String, i64), broadcast::Sender<Message>>>,
}

impl State {
    /// The fan-out of the conversation with the agent `template` and
    /// `index`: every message written to it goes to every page open on
    /// it.
    pub async fn room(&self, template: &str, index: i64) -> broadcast::Sender<Message> {
        let mut rooms = self.rooms.lock().await;
        rooms.entry((template.to_string(), index)).or_insert_with(|| broadcast::channel(ROOM).0).clone()
    }

    /// Tell every page open on the message's conversation. No page open
    /// is nothing.
    pub async fn told(&self, message: &Message) {
        let _ = self.room(&message.template, message.index).await.send(message.clone());
    }
}

/// The server one MCP session is served by, and what the frontend's
/// routes hold: every one a clone of one chat, so the state is the
/// container's.
#[derive(Clone, Default)]
pub struct Chat {
    /// The shared state.
    pub state: Arc<State>,
}

/// What a tool call has to work with: the shared state and the agent
/// the daemon attested.
pub struct Call<'a> {
    /// The shared state.
    pub state: &'a State,
    /// The agent calling.
    pub caller: Participant,
}

impl Chat {
    /// A chat, holding nothing yet.
    pub fn new() -> Chat {
        Chat::default()
    }
}

/// The instructions an agent's MCP client is handed on connecting.
const INSTRUCTIONS: &str = "A chat with a person. When the person writes to you, you are sent exactly `<chat>\\nYou have a new message\\n</chat>`. \
Read what the person wrote with `read_messages` — unread messages only, or every message with `include_read` — and answer with \
`send_message`. Your answers appear in the person's page at once.";

impl ServerHandler for Chat {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("chat", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools::all()))
    }

    /// Every call: the attested agent, then the tool by name. A tool's
    /// own failure is its error result, for the agent to read; only a
    /// tool the chat does not have is a protocol error.
    async fn call_tool(&self, request: CallToolRequestParams, context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        if !tools::NAMES.contains(&request.name.as_ref()) {
            return Err(ErrorData::invalid_params(format!("the chat has no tool named {}", request.name), None));
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
            let call = Call { state: &self.state, caller: agent(meta)? };
            tools::call(&request.name, call, request.arguments.unwrap_or_default()).await
        }
        .await;
        Ok(CallToolResponse::from(match outcome {
            Ok(value) => CallToolResult::structured(value),
            Err(failure) => CallToolResult::error(vec![rmcp::model::ContentBlock::text(failure.to_string())]),
        }))
    }
}
