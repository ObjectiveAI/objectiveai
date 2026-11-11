//! The MCP server: the tool's state, and the three tools served.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult, MetaObject, PaginatedRequestParams, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use tokio::sync::Notify;

use crate::caller::{Participant, caller};
use crate::daemon::Daemon;
use crate::store::Store;
use crate::tools;

/// What every MCP session and the scheduler share: the proxy's
/// client, the database pool and the daemon connection, each made on
/// first use, and the scheduler's wake.
#[derive(Default)]
pub struct State {
    /// The proxy, from the inside.
    pub client: Client,
    /// The alarms.
    pub store: Store,
    /// The daemon, for waking agents.
    pub daemon: Daemon,
    /// Signalled when an alarm is set or cancelled, so the scheduler
    /// sleeps until the right one.
    pub wake: Notify,
}

/// The server one MCP session is served by: every session the proxy
/// opens is a clone of one alarm tool, so the state is the
/// container's.
#[derive(Clone, Default)]
pub struct Alarm {
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

impl Alarm {
    /// An alarm tool, holding nothing yet.
    pub fn new() -> Alarm {
        Alarm::default()
    }

    /// The state, for the scheduler to share.
    pub fn state(&self) -> Arc<State> {
        Arc::clone(&self.state)
    }
}

/// The instructions an agent's MCP client is handed on connecting.
const INSTRUCTIONS: &str = "Alarms that wake you later. Set one with `set_alarm`: a note, and exactly one of `at` (an RFC 3339 time), \
`in_seconds`, or `cron` (a five-field pattern in UTC, for a recurring alarm). When it is due you are sent the note, as \
`<alarm>` the note `</alarm>`, even if you were not running. See yours with `list_alarms`; cancel one with `cancel_alarm`.";

impl ServerHandler for Alarm {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("alarm", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools::all()))
    }

    /// Every call: the attested caller, then the tool by name. A
    /// tool's own failure is its error result, for the agent to read;
    /// only a tool the alarm tool does not have is a protocol error.
    async fn call_tool(&self, request: CallToolRequestParams, context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        if !tools::NAMES.contains(&request.name.as_ref()) {
            return Err(ErrorData::invalid_params(format!("the alarm tool has no tool named {}", request.name), None));
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
