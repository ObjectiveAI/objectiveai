//! The room as a tool container image: the HTTP server a room program is,
//! on the container's loopback, answering the proxy beside it exactly as
//! Ronald's contract states (`diverge_sdk::container_proxy::inside::tool`):
//!
//! | the proxy calls | the program answers |
//! |-----------------|---------------------|
//! | `POST /register` | the room's `Args` (and, for a successor, the record it continues), once; `409` after |
//! | `GET /schema` | the JSON Schema of those arguments |
//! | `/mcp` | MCP over Streamable HTTP: the room's verbs, resources and notifications |
//!
//! What the room needs from its host (sealing a receipt, hearing a hire)
//! goes back through the proxy as the container's own `mcp-call-tool`,
//! which the provider relays to the runner: the host's app.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams, Resource, ReadResourceResponse, ServerCapabilities,
    ServerInfo, ServerNotification,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::{Value, json};

use diverge_sdk::container_proxy::inside::register;

use crate::room::{ABOUT, CHARTER, CHARTERS, DOORWAYS, FEED, MEMBERS, RECORD};
use crate::{Args, Host, Move, Room, Statement};

/// One running room program: the room once registered, and its host's side.
pub struct Program {
    room: Mutex<Option<Room>>,
    host: Arc<dyn Host>,
}

pub type Shared = Arc<Program>;

impl Program {
    pub fn new(host: Arc<dyn Host>) -> Shared {
        Arc::new(Program { room: Mutex::new(None), host })
    }

    fn with<T>(&self, f: impl FnOnce(&mut Room, &dyn Host) -> Result<T, ErrorData>) -> Result<T, ErrorData> {
        let mut guard = self.room.lock().unwrap_or_else(|p| p.into_inner());
        let room = guard.as_mut().ok_or_else(|| ErrorData::invalid_request("the room isn't registered yet", None))?;
        f(room, self.host.as_ref())
    }
}

/// The host's side, reached through the proxy: the room's own
/// `mcp-call-tool` to its runner, answered by the host's app.
pub struct ProxyHost {
    client: diverge_sdk::container_proxy::inside::Client,
}

impl ProxyHost {
    pub fn new() -> Self {
        ProxyHost { client: diverge_sdk::container_proxy::inside::Client::new() }
    }
}

impl Default for ProxyHost {
    fn default() -> Self {
        Self::new()
    }
}

/// The two tools a room asks of its host's app.
pub const HOST_SEAL: &str = "room_host_seal";
pub const HOST_HIRE: &str = "room_host_hire";

impl Host for ProxyHost {
    fn seal(&self, kind: &str, body: Value) -> Result<Statement, String> {
        let params = CallToolRequestParams::new(HOST_SEAL).with_arguments(json!({ "kind": kind, "body": body }).as_object().cloned().unwrap_or_default());
        // The room's verbs are synchronous; the proxy is not. This runs on the
        // multi-threaded runtime the program starts, so blocking here is allowed.
        let result = tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(self.client.mcp_call_tool(params))).map_err(|e| e.to_string())?;
        let text = result.content.iter().find_map(|c| c.as_text().map(|t| t.text.clone())).unwrap_or_default();
        if result.is_error == Some(true) {
            return Err(text);
        }
        serde_json::from_str(&text).map_err(|_| "the host's app answered with something that isn't a seal".to_string())
    }

    fn hire(&self, room: &str, hire_id: &str, from: &str, agent: &str, what: &str, pledge: Option<&str>) {
        let params = CallToolRequestParams::new(HOST_HIRE)
            .with_arguments(json!({ "room": room, "hire_id": hire_id, "from": from, "agent": agent, "what": what, "pledge": pledge }).as_object().cloned().unwrap_or_default());
        let client = diverge_sdk::container_proxy::inside::Client::new();
        tokio::spawn(async move {
            let _ = client.mcp_call_tool(params).await;
        });
    }
}

async fn register_room(State(program): State<Shared>, Json(request): Json<register::request::Request>) -> (StatusCode, Json<Value>) {
    let mut guard = program.room.lock().unwrap_or_else(|p| p.into_inner());
    if guard.is_some() {
        return (StatusCode::CONFLICT, Json(json!({ "kind": "registered" })));
    }
    let history: Vec<Move> = request.arguments.get("history").cloned().and_then(|h| serde_json::from_value(h).ok()).unwrap_or_default();
    let args: Args = match serde_json::from_value(request.arguments) {
        Ok(args) => args,
        Err(e) => return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({ "message": format!("these are not a room's settings: {e}") }))),
    };
    let made = if args.continues.is_some() { Room::from_record(args, history, &[], program.host.as_ref()) } else { Ok(Room::new(args)) };
    match made {
        Ok(room) => {
            *guard = Some(room);
            // A room deploys no tools of its own.
            (StatusCode::OK, Json(serde_json::to_value(register::response::Response::default()).unwrap_or_default()))
        }
        Err(message) => (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({ "message": message }))),
    }
}

/// What a room is started with.
pub fn args_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "A room",
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "title": { "type": "string" },
            "kind": { "type": "string", "enum": ["home", "board", "idea", "dm", "profile"] },
            "host_key": { "type": "string", "description": "The host's public key, as hex." },
            "host_name": { "type": "string" },
            "charter": { "type": "string", "description": "The room's rules, in Markdown." },
            "open_door": { "type": "boolean", "description": "Whether someone may knock without an invite." },
            "continues": { "type": "object", "description": "The room this one continues: its id, title, and the hash of its last move." },
            "history": { "type": "array", "description": "The record of the room this one continues." }
        },
        "required": ["id", "title", "kind", "host_key", "host_name", "charter"]
    })
}

async fn schema() -> Json<Value> {
    Json(args_schema())
}

/// The room's MCP server: one per session, all sharing the one room.
#[derive(Clone)]
pub struct Mcp(Shared);

impl ServerHandler for Mcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().enable_resources().build())
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        self.0.with(|room, _| Ok(room.tools()))
    }

    async fn call_tool(&self, mut request: CallToolRequestParams, context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        // rmcp lifts a request's `_meta` into its context; the seal rides
        // there, relayed untouched by the provider and the proxy.
        if request.meta.is_none() {
            request.meta = Some(context.meta.clone());
        }
        self.0.with(|room, host| room.call(request, host)).map(Into::into)
    }

    async fn list_resources(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListResourcesResult, ErrorData> {
        let resources = [FEED, MEMBERS, CHARTER, CHARTERS, DOORWAYS, ABOUT, RECORD].into_iter().map(|uri| Resource::new(uri, uri.trim_start_matches("space://"))).collect();
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn read_resource(&self, request: ReadResourceRequestParams, _context: RequestContext<RoleServer>) -> Result<ReadResourceResponse, ErrorData> {
        self.0.with(|room, _| room.read(&request.uri)).map(Into::into)
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        // Every change the room announces goes to this session; the proxy
        // fans it out to every member.
        let Ok(mut live) = self.0.with(|room, _| Ok(room.subscribe())) else { return };
        tokio::spawn(async move {
            while let Ok(n) = live.recv().await {
                if let ServerNotification::ResourceUpdatedNotification(n) = n {
                    if context.peer.notify_resource_updated(n.params).await.is_err() {
                        return;
                    }
                }
            }
        });
    }
}

/// The whole program: `/register`, `/schema`, `/mcp`.
pub fn router(program: Shared) -> Router {
    let mcp: StreamableHttpService<Mcp, LocalSessionManager> = {
        let program = program.clone();
        StreamableHttpService::new(move || Ok(Mcp(program.clone())), Default::default(), StreamableHttpServerConfig::default())
    };
    Router::new().route("/register", post(register_room)).route("/schema", get(schema)).nest_service("/mcp", mcp).with_state(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Keypair, seal_call};
    use rmcp::ServiceExt;
    use rmcp::model::ResourceContents;
    use rmcp::transport::StreamableHttpClientTransport;

    struct TestHost(Keypair);

    impl Host for TestHost {
        fn seal(&self, kind: &str, body: Value) -> Result<Statement, String> {
            Ok(Statement::make(&self.0, kind, body))
        }
    }

    /// The program answers the proxy's three calls as Ronald's contract
    /// states, and a sealed call over real MCP lands in the room.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_program_answers_the_proxys_three_calls() {
        let host = Keypair::from_seed("host");
        let program = Program::new(Arc::new(TestHost(host.clone())));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router(program)).await.unwrap() });
        let http = reqwest::Client::new();
        let args = json!({ "id": "room-1", "title": "A workshop", "kind": "board", "host_key": host.key(), "host_name": "maya", "charter": "# Rules" });

        let first = http.post(format!("http://{addr}/register")).json(&json!({ "arguments": args })).send().await.unwrap();
        assert!(first.status().is_success());
        let body: register::response::Response = first.json().await.unwrap();
        assert!(body.tools.is_empty(), "a room deploys no tools");
        let second = http.post(format!("http://{addr}/register")).json(&json!({ "arguments": args })).send().await.unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT);
        let schema: Value = http.get(format!("http://{addr}/schema")).send().await.unwrap().json().await.unwrap();
        assert_eq!(schema["properties"]["kind"]["enum"][1], "board");

        let client = ().serve(StreamableHttpClientTransport::from_uri(format!("http://{addr}/mcp"))).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();
        assert!(tools.iter().any(|t| t.name == "post_task"));
        let mut params = CallToolRequestParams::new("post_task").with_arguments(json!({ "title": "Fix the lamp", "spec": "It turns on." }).as_object().cloned().unwrap());
        seal_call(&host, "room-1", &mut params, 1);
        let result = client.call_tool(params).await.unwrap();
        assert_ne!(result.is_error, Some(true));
        let unsealed = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        assert!(client.call_tool(unsealed).await.is_err(), "the room refuses an unsealed call");
        let feed = client.read_resource(ReadResourceRequestParams::new(FEED)).await.unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &feed.contents[0] else { panic!() };
        assert!(text.contains("Fix the lamp"));
        let _ = client.cancel().await;
    }
}
