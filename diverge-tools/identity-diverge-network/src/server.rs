//! The MCP server: the tool's state, and the two tools served.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::daemon::Daemon;
use crate::failure::Failure;
use crate::register;
use crate::store::Store;
use crate::tools;

/// What every MCP session shares: the proxy's client, the database
/// pool and the daemon connection, each made on first use, and the
/// client the issuers and `identity.diverge.network` are asked with.
pub struct State {
    /// The proxy, from the inside.
    pub client: Client,
    /// The accounts.
    pub store: Store,
    /// The daemon, for the login.
    pub daemon: Daemon,
    /// The HTTP client. It follows no redirect, so no answer can send
    /// a request, and the token it carries, elsewhere.
    pub http: reqwest::Client,
}

impl Default for State {
    fn default() -> Self {
        // A client that cannot be built with no redirects is built as
        // the default is: the tool serves either way, and a token is
        // only ever sent to the two addresses this crate names.
        let http = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).user_agent("diverge").build().unwrap_or_default();
        State { client: Client::default(), store: Store::default(), daemon: Daemon::default(), http }
    }
}

/// The server one MCP session is served by: every session the proxy
/// opens is a clone of one server, so the state is the container's.
#[derive(Clone, Default)]
pub struct Server {
    state: Arc<State>,
}

/// What a tool call has to work with: the shared state, and the
/// template the accounts are kept under.
pub struct Call<'a> {
    /// The shared state.
    pub state: &'a State,
    /// The template the tool was made from.
    pub template: &'a str,
}

impl Server {
    /// A server, holding nothing yet.
    pub fn new() -> Server {
        Server::default()
    }
}

/// The instructions a caller's MCP client is handed on connecting.
const INSTRUCTIONS: &str = "Signs identities in on the daemon's host at Diverge's own issuer, https://identity.diverge.network, with nobody entering a code. \
See which accounts this tool holds with `list_accounts`: each is `<issuer>.<id>`, a GitHub or Google sign-in, and owns every subject that begins with it. \
Sign a subject in with `login`, giving the account's `issuer` and `id` and the subject's name as `subject`: the whole subject is `<issuer>.<id>.<subject>`. \
A name never used before begins a new subject. No account can be added to this tool yet, so `list_accounts` may answer none.";

impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("identity-diverge-network", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools::all()))
    }

    /// Every call: the registration, then the tool by name. A tool's
    /// own failure is its error result, for the caller to read; only a
    /// tool the server does not have is a protocol error. Whoever the
    /// daemon lets call the tool may: what a caller may do with it is
    /// the daemon's to say, by who is attached to it.
    async fn call_tool(&self, request: CallToolRequestParams, _context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        if !tools::NAMES.contains(&request.name.as_ref()) {
            return Err(ErrorData::invalid_params(format!("this tool has no tool named {}", request.name), None));
        }
        let outcome = async {
            let template = register::registered().ok_or(Failure::Unregistered)?.template.as_str();
            let call = Call { state: &self.state, template };
            tools::call(&request.name, call, request.arguments.unwrap_or_default()).await
        }
        .await;
        Ok(CallToolResponse::from(match outcome {
            Ok(value) => CallToolResult::structured(value),
            Err(failure) => CallToolResult::error(vec![rmcp::model::ContentBlock::text(failure.to_string())]),
        }))
    }
}
