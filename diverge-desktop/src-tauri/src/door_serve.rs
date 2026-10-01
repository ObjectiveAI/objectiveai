//! The agent door, served on this machine only, for local agents you
//! already run yourself (Claude Code, say).
//!
//! The door ([`crate::door::Door`]) is the same one the daemon's agents
//! reach; here it is an MCP server over Streamable HTTP on 127.0.0.1, on a
//! port picked once and kept. It listens only while at least one local
//! agent is added, and only in the copy of the app that holds its folder.
//!
//! - **One token per local agent.** It lives in an owner-only file in the
//!   data folder (`door/<id>.token`), beside a tiny helper (`door/<id>-headers`)
//!   that prints the header a client sends. The connect line names the
//!   helper, never the token, and the page never sees the token at all. The
//!   app keeps only each token's sha256 (`door_agents.json`).
//! - **Which agent is calling comes only from which token matched**,
//!   compared in constant time against every agent's. Nothing the client
//!   says about itself counts. A session is bound to the agent that opened it.
//! - **A new key stops the old one at once**: the next request with it is
//!   refused, and calls still waiting on a card under it are withdrawn.
//! - **Browsers are refused**: any request carrying an `Origin` header is
//!   turned away, and rmcp's own check keeps the `Host` to the loopback names.
//! - **The port never moves on its own.** If something else holds it, the
//!   door says so and doesn't listen.

use std::collections::{BTreeMap, HashMap};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use rmcp::model::{CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio_util::sync::CancellationToken;

use crate::door::Door;
use crate::identity::{AgentId, AgentKind, Identity};
use crate::view::{DoorState, DoorStatusView, LocalAgentView};

/// The file that keeps each local agent's token as a sha256, and the door's port.
pub const DOOR_AGENTS_FILE: &str = "door_agents.json";
/// The folder, in the data folder, that holds each local agent's token and helper.
pub const DOOR_DIR: &str = "door";
/// How long Claude Code waits on one call through the door, in milliseconds:
/// a day, so a card can wait on you. Claude Code's own limit otherwise is
/// minutes; this is a starting position, yours to change in its settings.
pub const CALL_TIMEOUT_MS: u64 = 86_400_000;

/// What an action gets when this copy of the app doesn't hold its folder.
pub const NOT_THIS_COPY: &str = "This copy of the app doesn't hold its folder, so it serves no door and adds no agents.";
/// What an action gets for a local agent that isn't added.
pub const NO_SUCH_AGENT: &str = "No local agent goes by that id here.";

#[derive(Serialize, Deserialize, Default, Clone)]
struct Kept {
    /// The port the door was given once.
    port: Option<u16>,
    /// Each local agent the door is served to, by id.
    agents: BTreeMap<String, KeptAgent>,
}

#[derive(Serialize, Deserialize, Clone)]
struct KeptAgent {
    /// The sha256 of its token, as hex. The token itself is only in its file.
    token_sha256: String,
    added: DateTime<Utc>,
}

/// Who a request came from: the id whose token it carried. Put there by the
/// door's own check, never by anything the client sends.
#[derive(Clone, Debug)]
struct Caller(String);

struct Listening {
    stop: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}

pub struct DoorServe {
    door: Arc<Door>,
    identity: Arc<Identity>,
    data: PathBuf,
    /// Whether this copy holds its folder: one that doesn't serves nothing.
    held: bool,
    kept: Mutex<Kept>,
    listening: Mutex<Option<Listening>>,
    port_taken: Mutex<bool>,
    /// Each agent's calls in flight hang under its token: a new key or a removal cancels them.
    calls: Mutex<HashMap<String, CancellationToken>>,
    /// Each MCP session, by its id: the agent that opened it, and its calls in
    /// flight, cancelled when the client ends the session.
    sessions: Mutex<HashMap<String, (String, CancellationToken)>>,
    rt: tokio::runtime::Handle,
}

fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// A new token: 32 random bytes, as hex.
fn new_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| e.to_string())?;
    Ok(hex::encode(bytes))
}

/// Write a file only its owner can use, whole: to a new file beside it, then swapped in.
fn write_private(path: &Path, bytes: &[u8], mode: u32) -> std::io::Result<()> {
    use std::io::Write;
    let dir = path.parent().ok_or_else(|| std::io::Error::other("no folder"))?;
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let tmp = dir.join(format!(".{}.{}.tmp", path.file_name().and_then(|n| n.to_str()).unwrap_or("file"), std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    let written = (|| {
        let mut f = options.open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

/// A path inside single quotes, for `sh`.
fn sh_single(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

/// A path inside double quotes, for `sh`: what Claude Code runs a helper with.
fn sh_double(path: &Path) -> String {
    let mut out = String::from("\"");
    for c in path.to_string_lossy().chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// The helper that prints the header one local agent sends: it reads the
/// token from the file beside it, so the token is in no command line.
fn helper_script(token_file: &Path) -> String {
    format!(
        "#!/bin/sh\n# Prints the header this app's door expects from one local agent. The token is read from its own file.\nt=$(cat {}) || exit 1\nprintf '{{\"Authorization\":\"Bearer %s\"}}\\n' \"$t\"\n",
        sh_single(token_file)
    )
}

/// The line that adds a local agent to Claude Code, for you: it names the
/// door's address and the helper, never the token.
pub fn connect_line(id: &str, port: u16, helper: &Path) -> String {
    let config = serde_json::json!({
        "type": "http",
        "url": format!("http://127.0.0.1:{port}/mcp"),
        "headersHelper": sh_double(helper),
        "timeout": CALL_TIMEOUT_MS,
    });
    let json = serde_json::to_string(&config).unwrap_or_default();
    format!("claude mcp add-json diverge-{id} '{}' --scope user", json.replace('\'', "'\\''"))
}

/// An id from a name: lowercase letters, digits and dashes.
fn id_from(name: &str) -> String {
    let mut id = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            id.push(c);
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id: String = id.trim_end_matches('-').chars().take(32).collect();
    let id = id.trim_end_matches('-').to_owned();
    if id.is_empty() { "agent".into() } else { id }
}

impl DoorServe {
    /// The door's local agents as `data` keeps them, listening at once if any
    /// is added and this copy holds its folder. Call it inside a tokio runtime.
    pub fn open(data: PathBuf, door: Arc<Door>, identity: Arc<Identity>, held: bool) -> Arc<Self> {
        let file = data.join(DOOR_AGENTS_FILE);
        let kept: Kept = if held { crate::store::load(&file, crate::store::DOOR_AGENTS) } else { crate::store::peek(&file, crate::store::DOOR_AGENTS) }.unwrap_or_default();
        let serve = Arc::new(DoorServe {
            door,
            identity,
            data,
            held,
            kept: Mutex::new(kept),
            listening: Mutex::new(None),
            port_taken: Mutex::new(false),
            calls: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            rt: tokio::runtime::Handle::current(),
        });
        serve.listen();
        serve
    }

    fn token_file(&self, id: &str) -> PathBuf {
        self.data.join(DOOR_DIR).join(format!("{id}.token"))
    }

    /// The helper that prints one local agent's header.
    pub fn helper_file(&self, id: &str) -> PathBuf {
        self.data.join(DOOR_DIR).join(format!("{id}-headers"))
    }

    fn save(&self, kept: &Kept) -> Result<(), String> {
        crate::store::save(&self.data.join(DOOR_AGENTS_FILE), crate::store::DOOR_AGENTS, kept).map_err(|e| e.to_string())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Kept> {
        self.kept.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The id whose token this is, compared in constant time against every
    /// agent's, so how long it takes says nothing about which came close.
    fn matching(&self, presented: &str) -> Option<String> {
        let presented = digest(presented);
        let kept = self.lock();
        let mut found = None;
        for (id, a) in &kept.agents {
            let Ok(stored) = hex::decode(&a.token_sha256) else { continue };
            if bool::from(stored.ct_eq(&presented[..])) {
                found = Some(id.clone());
            }
        }
        found
    }

    /// Calls in flight for one agent hang under this.
    fn calls_of(&self, id: &str) -> CancellationToken {
        self.calls.lock().unwrap_or_else(|p| p.into_inner()).entry(id.to_owned()).or_default().clone()
    }

    /// Cancel every call one agent has in flight, and forget its sessions.
    fn cut_off(&self, id: &str) {
        if let Some(token) = self.calls.lock().unwrap_or_else(|p| p.into_inner()).remove(id) {
            token.cancel();
        }
        self.sessions.lock().unwrap_or_else(|p| p.into_inner()).retain(|_, (owner, calls)| {
            if owner == id {
                calls.cancel();
            }
            owner != id
        });
    }

    /// Give an agent a new token: its file and helper are written, and only
    /// then is its sha256 kept, so the old one stops the moment the new one works.
    fn issue(&self, id: &str, kept: &mut Kept) -> Result<(), String> {
        let token = new_token()?;
        write_private(&self.token_file(id), token.as_bytes(), 0o600).map_err(|e| e.to_string())?;
        write_private(&self.helper_file(id), helper_script(&self.token_file(id)).as_bytes(), 0o700).map_err(|e| e.to_string())?;
        let added = kept.agents.get(id).map(|a| a.added).unwrap_or_else(Utc::now);
        kept.agents.insert(id.to_owned(), KeptAgent { token_sha256: hex::encode(digest(&token)), added });
        Ok(())
    }

    fn view(&self, id: &str, kept: &Kept) -> Option<LocalAgentView> {
        let a = kept.agents.get(id)?;
        Some(LocalAgentView {
            id: id.to_owned(),
            name: self.identity.local_name(id).unwrap_or_else(|| id.to_owned()),
            slot: AgentId::Local(id.to_owned()).slot(),
            connect_line: kept.port.map(|port| connect_line(id, port, &self.helper_file(id))),
            added: a.added.to_rfc3339(),
        })
    }

    /// The local agents the door is served to.
    pub fn list(&self) -> Vec<LocalAgentView> {
        let kept = self.lock();
        kept.agents.keys().filter(|id| self.identity.is_local(id)).filter_map(|id| self.view(id, &kept)).collect()
    }

    /// Add a local agent under a name you give it: its own keys, a token
    /// in its own file, and the line that adds it to Claude Code. The first
    /// one gives the door its port, kept from then on.
    pub fn add(self: &Arc<Self>, name: &str) -> Result<LocalAgentView, String> {
        if !self.held {
            return Err(NOT_THIS_COPY.into());
        }
        self.identity.ready()?;
        let name = name.trim();
        if name.is_empty() {
            return Err("a local agent needs a name".into());
        }
        let mut kept = self.lock();
        let base = id_from(name);
        let mut id = base.clone();
        let mut n = 2;
        while !self.identity.local_id_free(&id) || kept.agents.contains_key(&id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        let mut next = kept.clone();
        if next.port.is_none() {
            let probe = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
            next.port = Some(probe.local_addr().map_err(|e| e.to_string())?.port());
        }
        self.issue(&id, &mut next)?;
        self.identity.add_local(&id, name)?;
        self.save(&next)?;
        *kept = next;
        let view = self.view(&id, &kept);
        drop(kept);
        self.listen();
        view.ok_or_else(|| NO_SUCH_AGENT.into())
    }

    /// A new key for a local agent: the old one stops at once, and whatever
    /// it was waiting on is withdrawn. The helper prints the new one.
    pub fn new_key(&self, id: &str) -> Result<LocalAgentView, String> {
        if !self.held {
            return Err(NOT_THIS_COPY.into());
        }
        let mut kept = self.lock();
        if !kept.agents.contains_key(id) || !self.identity.is_local(id) {
            return Err(NO_SUCH_AGENT.into());
        }
        let mut next = kept.clone();
        self.issue(id, &mut next)?;
        self.save(&next)?;
        *kept = next;
        drop(kept);
        self.cut_off(id);
        let kept = self.lock();
        self.view(id, &kept).ok_or_else(|| NO_SUCH_AGENT.into())
    }

    /// Remove a local agent: its token and helper are gone, it acts no
    /// more, and the door stops listening once none is left.
    pub async fn remove(&self, id: &str) -> Result<(), String> {
        if !self.held {
            return Err(NOT_THIS_COPY.into());
        }
        {
            let mut kept = self.lock();
            if !kept.agents.contains_key(id) {
                return Err(NO_SUCH_AGENT.into());
            }
            let mut next = kept.clone();
            next.agents.remove(id);
            self.save(&next)?;
            *kept = next;
        }
        self.cut_off(id);
        let _ = std::fs::remove_file(self.token_file(id));
        let _ = std::fs::remove_file(self.helper_file(id));
        if self.identity.is_local(id) {
            self.identity.remove_local(id)?;
        }
        if self.lock().agents.is_empty() {
            self.stop().await;
        }
        Ok(())
    }

    /// Whether the door is listening, and on which port. If its port was
    /// taken, it tries that same port again.
    pub fn status(self: &Arc<Self>) -> DoorStatusView {
        self.listen();
        let port = self.lock().port;
        let state = if !self.held {
            DoorState::NotThisCopy
        } else if self.lock().agents.is_empty() {
            DoorState::NoAgents
        } else if self.listening.lock().unwrap_or_else(|p| p.into_inner()).is_some() {
            DoorState::Listening
        } else {
            DoorState::PortTaken
        };
        DoorStatusView { port, state }
    }

    /// Listen on the door's own port, if an agent is added and this copy
    /// holds its folder. A port something else holds is said, never moved.
    fn listen(self: &Arc<Self>) {
        if !self.held || self.lock().agents.is_empty() {
            return;
        }
        let mut listening = self.listening.lock().unwrap_or_else(|p| p.into_inner());
        if listening.is_some() {
            return;
        }
        let Some(port) = self.lock().port else { return };
        let _entered = self.rt.enter();
        let bound = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).and_then(|l| {
            l.set_nonblocking(true)?;
            tokio::net::TcpListener::from_std(l)
        });
        let listener = match bound {
            Ok(l) => l,
            Err(_) => {
                *self.port_taken.lock().unwrap_or_else(|p| p.into_inner()) = true;
                return;
            }
        };
        *self.port_taken.lock().unwrap_or_else(|p| p.into_inner()) = false;
        let stop = CancellationToken::new();
        let app = self.router(stop.clone());
        let shutdown = stop.clone();
        let task = self.rt.spawn(async move {
            let _ = axum::serve(listener, app).with_graceful_shutdown(shutdown.cancelled_owned()).await;
        });
        *listening = Some(Listening { stop, task });
    }

    /// Stop listening, and let go of the port.
    pub async fn stop(&self) {
        let was = self.listening.lock().unwrap_or_else(|p| p.into_inner()).take();
        if let Some(l) = was {
            l.stop.cancel();
            l.task.abort();
            let _ = l.task.await;
        }
    }

    fn router(self: &Arc<Self>, stop: CancellationToken) -> Router {
        let me = self.clone();
        let mcp: StreamableHttpService<Mcp, LocalSessionManager> = {
            let me = me.clone();
            // rmcp's own Host check stays: only the loopback names.
            StreamableHttpService::new(move || Ok(Mcp(me.clone())), Default::default(), StreamableHttpServerConfig::default().with_cancellation_token(stop))
        };
        Router::new().nest_service("/mcp", mcp).layer(middleware::from_fn_with_state(me, guard))
    }
}

/// The door's check on every request: no browser, a token that matches
/// one agent's, and a session only that agent opened.
async fn guard(State(serve): State<Arc<DoorServe>>, mut request: Request<Body>, next: Next) -> Response {
    if request.headers().contains_key(header::ORIGIN) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let presented = request.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).map(str::trim);
    let Some(id) = presented.and_then(|t| serve.matching(t)) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let session = request.headers().get("mcp-session-id").and_then(|v| v.to_str().ok()).map(str::to_owned);
    if let Some(session) = &session {
        if serve.sessions.lock().unwrap_or_else(|p| p.into_inner()).get(session).map(|(owner, _)| owner) != Some(&id) {
            return StatusCode::NOT_FOUND.into_response();
        }
    }
    let ending = request.method() == axum::http::Method::DELETE;
    request.extensions_mut().insert(Caller(id.clone()));
    let response = next.run(request).await;
    match &session {
        // The client ended its session: whatever it was still waiting on is withdrawn.
        Some(session) if ending => {
            if let Some((_, calls)) = serve.sessions.lock().unwrap_or_else(|p| p.into_inner()).remove(session) {
                calls.cancel();
            }
        }
        Some(_) => {}
        None => {
            if let Some(opened) = response.headers().get("mcp-session-id").and_then(|v| v.to_str().ok()) {
                serve.sessions.lock().unwrap_or_else(|p| p.into_inner()).insert(opened.to_owned(), (id, CancellationToken::new()));
            }
        }
    }
    response
}

/// The door as one session's MCP server.
#[derive(Clone)]
struct Mcp(Arc<DoorServe>);

impl Mcp {
    /// The session this request rides on, if the client opened one: its calls hang under it.
    fn session_calls(&self, context: &RequestContext<RoleServer>) -> CancellationToken {
        let session = context.extensions.get::<axum::http::request::Parts>().and_then(|p| p.headers.get("mcp-session-id")).and_then(|v| v.to_str().ok());
        session.and_then(|s| self.0.sessions.lock().unwrap_or_else(|p| p.into_inner()).get(s).map(|(_, calls)| calls.clone())).unwrap_or_default()
    }

    /// The local agent this request came from, by its token alone.
    fn caller(context: &RequestContext<RoleServer>) -> Result<AgentId, ErrorData> {
        context
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|p| p.extensions.get::<Caller>())
            .map(|c| AgentId::Local(c.0.clone()))
            .ok_or_else(|| ErrorData::invalid_request(crate::door::UNKNOWN_AGENT, None))
    }
}

impl ServerHandler for Mcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(Implementation::new("diverge", env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, context: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        Self::caller(&context)?;
        Ok(self.0.door.tools(AgentKind::Local))
    }

    async fn call_tool(&self, request: CallToolRequestParams, context: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        let agent = Self::caller(&context)?;
        let AgentId::Local(id) = &agent else { return Err(ErrorData::invalid_request(crate::door::UNKNOWN_AGENT, None)) };
        // The call ends when the client cancels it or ends its session, or when its key is replaced or removed.
        let cancel = self.0.calls_of(id).child_token();
        let watcher = {
            let (cancel, client, session) = (cancel.clone(), context.ct.clone(), self.session_calls(&context));
            tokio::spawn(async move {
                tokio::select! {
                    _ = client.cancelled() => cancel.cancel(),
                    _ = session.cancelled() => cancel.cancel(),
                    _ = cancel.cancelled() => {}
                }
            })
        };
        let result = self.0.door.call(&agent, request, cancel).await;
        watcher.abort();
        result.map(Into::into)
    }
}

#[cfg(all(test, feature = "stand-in"))]
mod tests {
    use super::*;
    use std::time::Duration;

    use futures::StreamExt;
    use rmcp::ServiceExt;
    use rmcp::model::{CallToolResult, ClientRequest};
    use rmcp::service::{PeerRequestOptions, RunningService};
    use rmcp::transport::StreamableHttpClientTransport;
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
    use rmcp::RoleClient;
    use serde_json::{Value, json};

    use crate::daemon::stub::StubDaemon;
    use crate::door::{Reach, YES};
    use crate::spaces::stub::{StubSpaces, board};
    use crate::spaces::{Id, Spaces};
    use crate::view::CardEvent;
    use diverge_desktop_room::Keypair;

    /// The app's door over a folder of its own: keys made from the stand-in's
    /// seed (so its rooms have the ids every test's do), its rooms, and the
    /// door served from it.
    struct Here {
        identity: Arc<Identity>,
        stub: StubSpaces,
        door: Arc<Door>,
        serve: Arc<DoorServe>,
    }

    fn open_in(data: &Path) -> Here {
        let keys = data.join("identity.json");
        if !keys.exists() {
            let seeded = json!({ "personas": [{ "id": "usual", "name": "juno", "secret": Keypair::from_seed("juno").secret_hex(), "created": Utc::now(), "usual": true }], "agents": {}, "rooms": {} });
            crate::store::save(&keys, crate::store::KEYS, &seeded).unwrap();
            Identity::open(keys.clone()).finish_first_run("juno", true).unwrap();
        }
        let identity = Arc::new(Identity::open(keys));
        let stub = StubSpaces::new(identity.clone(), data.join("tables"));
        let daemon = StubDaemon::new(data.join("host"));
        let door = Arc::new(Door::new(Arc::new(stub.clone()), identity.clone(), Arc::new(daemon), Some(data.join("allowances.json"))));
        let serve = DoorServe::open(data.to_path_buf(), door.clone(), identity.clone(), true);
        Here { identity, stub, door, serve }
    }

    fn token_of(serve: &DoorServe, id: &str) -> String {
        std::fs::read_to_string(serve.token_file(id)).unwrap()
    }

    fn url(serve: &DoorServe) -> String {
        format!("http://127.0.0.1:{}/mcp", serve.lock().port.unwrap())
    }

    async fn client(serve: &DoorServe, token: &str) -> RunningService<RoleClient, ()> {
        let config = StreamableHttpClientTransportConfig::with_uri(url(serve)).auth_header(token.to_owned());
        ().serve(StreamableHttpClientTransport::from_config(config)).await.expect("the door answers a known token")
    }

    fn call(name: &str, args: Value) -> CallToolRequestParams {
        CallToolRequestParams::new(name.to_owned()).with_arguments(args.as_object().cloned().unwrap())
    }

    fn words(r: &CallToolResult) -> String {
        r.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n")
    }

    async fn result<S: rmcp::service::Service<RoleClient>>(c: &RunningService<RoleClient, S>, p: CallToolRequestParams) -> CallToolResult {
        c.call_tool(p).await.expect("an answer")
    }

    /// Let one of your local agents into the board, as its host does.
    async fn admit(h: &Here, room: &str, id: &str) {
        let agent = AgentId::Local(id.to_owned());
        let a = h.identity.agent_in(&agent, Some(room)).unwrap();
        let person = h.identity.who_in(room).unwrap();
        let mut p = call("admit", json!({ "key": a.key, "name": a.name, "is_agent": true, "agent_of": person.key, "tether": a.tether }));
        h.identity.seal(&h.identity.you_in(room), room, &mut p).unwrap();
        let r = h.stub.call(&Id { id: room.to_owned() }, p).await.unwrap();
        assert_ne!(r.is_error, Some(true), "{}", words(&r));
    }

    async fn moves(h: &Here, room: &str) -> Vec<Value> {
        let r = h.stub.read(&Id { id: room.to_owned() }, diverge_desktop_room::room::FEED).await.unwrap();
        let text = r.contents.iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    async fn next_card(events: &mut crate::daemon::Frames<CardEvent>) -> crate::view::CardView {
        loop {
            match tokio::time::timeout(Duration::from_secs(10), events.next()).await.expect("a card") {
                Some(CardEvent::Card { card }) => return card,
                Some(_) => continue,
                None => panic!("the cards ended"),
            }
        }
    }

    /// A plain POST to the door, as anything on this machine could send it.
    async fn raw(serve: &DoorServe, headers: &[(&str, &str)]) -> reqwest::StatusCode {
        let mut r = reqwest::Client::new()
            .post(url(serve))
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .body(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "probe", "version": "1" } } }).to_string());
        for (k, v) in headers {
            r = r.header(*k, *v);
        }
        r.send().await.unwrap().status()
    }

    /// Token A's call reaches a card; on a yes, the move is sealed by A's own
    /// key, tethered to you. Nothing the client says about itself counts.
    #[tokio::test(flavor = "multi_thread")]
    async fn token_a_calls_a_card_comes_up_and_on_a_yes_the_move_is_sealed_by_as_key() {
        let dir = crate::store::tests::folder("door-serve-a");
        let h = open_in(&dir);
        let room = board();
        let a = h.serve.add("Claude Code").unwrap();
        let b = h.serve.add("Another helper").unwrap();
        assert_ne!(a.id, b.id);
        admit(&h, &room, &a.id).await;
        let mut events = h.door.watch(CancellationToken::new());
        // B's client claims to be A; it's sealed as B, whatever it says.
        let ca = client(&h.serve, &token_of(&h.serve, &a.id)).await;
        let tools = ca.list_all_tools().await.unwrap();
        assert!(tools.iter().any(|t| t.name == "space_call") && !serde_json::to_string(&tools).unwrap().contains("credential"), "a local agent's tools");
        let calling = tokio::spawn(async move {
            let r = result(&ca, call("space_call", json!({ "space": board(), "tool": "show", "arguments": { "title": "A sketch from A" } }))).await;
            (r, ca)
        });
        let card = next_card(&mut events).await;
        assert_eq!(card.agent, format!("local/{}", a.id));
        assert_eq!(card.call.as_ref().map(|c| c.verb.as_str()), Some("show"));
        h.door.answer(card.id, YES.into()).unwrap();
        let (r, ca) = tokio::time::timeout(Duration::from_secs(10), calling).await.unwrap().unwrap();
        assert_ne!(r.is_error, Some(true), "{}", words(&r));
        let a_key = h.identity.agent_key(&AgentId::Local(a.id.clone()), &room).unwrap();
        let shown = moves(&h, &room).await.into_iter().find(|m| m["title"] == "A sketch from A").expect("the move is in the room");
        assert_eq!(shown["by"], a_key.as_str(), "sealed by A's own key");
        let members = h.stub.read(&Id { id: room.clone() }, diverge_desktop_room::room::MEMBERS).await.unwrap();
        let members: Vec<Value> = serde_json::from_str(&members.contents.iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }).unwrap()).unwrap();
        let you = h.identity.who_in(&room).unwrap();
        let tethered = members.iter().find(|m| m["key"] == a_key.as_str()).expect("A is a member");
        assert!(tethered["agent_of_key"] == you.key.as_str() || Some(tethered["agent_of_key"].as_str().unwrap_or_default()) == you.account.as_deref(), "tethered to you: {tethered}");
        let _ = ca.cancel().await;

        // B is in no room; claiming to be A changes nothing.
        let config = StreamableHttpClientTransportConfig::with_uri(url(&h.serve)).auth_header(token_of(&h.serve, &b.id));
        let info = rmcp::model::ClientInfo::new(Default::default(), rmcp::model::Implementation::new(format!("local/{}", a.id), "1"));
        let cb = info.serve(StreamableHttpClientTransport::from_config(config)).await.unwrap();
        let r = result(&cb, call("space_call", json!({ "space": room, "tool": "show", "arguments": { "title": "B posing as A" } }))).await;
        assert_eq!((r.is_error, words(&r)), (Some(true), crate::door::NOT_IN.to_owned()), "B is refused as B");
        assert!(h.door.cards().is_empty(), "and nobody was asked");
        assert!(!moves(&h, &room).await.iter().any(|m| m["title"] == "B posing as A"));
        let _ = cb.cancel().await;
        h.serve.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// No token, a wrong one, or one replaced by a new key: 401, and nothing happens.
    #[tokio::test(flavor = "multi_thread")]
    async fn no_token_a_wrong_one_or_a_replaced_one_gets_401_and_nothing_happens() {
        let dir = crate::store::tests::folder("door-serve-401");
        let h = open_in(&dir);
        let a = h.serve.add("Claude Code").unwrap();
        assert_eq!(raw(&h.serve, &[]).await, reqwest::StatusCode::UNAUTHORIZED);
        assert_eq!(raw(&h.serve, &[("authorization", "Bearer 0000")]).await, reqwest::StatusCode::UNAUTHORIZED);
        let old = token_of(&h.serve, &a.id);
        assert_eq!(raw(&h.serve, &[("authorization", &format!("Bearer {old}"))]).await, reqwest::StatusCode::OK);
        // A session open under the old token, a card waiting on it, then a new key.
        let room = board();
        admit(&h, &room, &a.id).await;
        let mut events = h.door.watch(CancellationToken::new());
        let ca = client(&h.serve, &old).await;
        let calling = tokio::spawn(async move { ca.call_tool(call("space_call", json!({ "space": board(), "tool": "show", "arguments": { "title": "under the old key" } }))).await });
        let card = next_card(&mut events).await;
        let renewed = h.serve.new_key(&a.id).unwrap();
        assert_eq!(renewed.connect_line, a.connect_line, "the line names the helper, so it stays the same");
        match tokio::time::timeout(Duration::from_secs(10), events.next()).await.unwrap() {
            Some(CardEvent::Withdrawn { id }) => assert_eq!(id, card.id, "what it was waiting on is withdrawn"),
            other => panic!("expected the card withdrawn, got {other:?}"),
        }
        let _ = tokio::time::timeout(Duration::from_secs(10), calling).await;
        assert_eq!(h.door.answer(card.id, YES.into()).unwrap_err(), crate::door::WITHDRAWN, "a late yes does nothing");
        assert_eq!(raw(&h.serve, &[("authorization", &format!("Bearer {old}"))]).await, reqwest::StatusCode::UNAUTHORIZED, "the old key stops at once");
        let new = token_of(&h.serve, &a.id);
        assert_ne!(new, old);
        assert_eq!(raw(&h.serve, &[("authorization", &format!("Bearer {new}"))]).await, reqwest::StatusCode::OK);
        assert!(!moves(&h, &room).await.iter().any(|m| m["title"] == "under the old key"), "nothing was said");
        // Removed: its token stops, its files are gone, and it acts no more.
        h.serve.remove(&a.id).await.unwrap();
        assert!(!h.serve.token_file(&a.id).exists() && !h.serve.helper_file(&a.id).exists());
        assert!(!h.identity.is_local(&a.id));
        assert_eq!(h.serve.status().state, DoorState::NoAgents, "and with none left, nothing listens");
        assert!(reqwest::Client::new().post(url(&h.serve)).bearer_auth(&new).send().await.is_err(), "nothing answers on the port");
        let again = h.serve.add("Claude Code").unwrap();
        assert_ne!(again.id, a.id, "a removed agent's id is never given to another");
        h.serve.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A browser can't reach the door: any Origin is refused, and so is a Host
    /// that isn't this machine's loopback, even with a good token.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_origin_or_a_foreign_host_is_refused_with_403() {
        let dir = crate::store::tests::folder("door-serve-403");
        let h = open_in(&dir);
        let a = h.serve.add("Claude Code").unwrap();
        let bearer = format!("Bearer {}", token_of(&h.serve, &a.id));
        assert_eq!(raw(&h.serve, &[("authorization", &bearer), ("host", "evil.example")]).await, reqwest::StatusCode::FORBIDDEN);
        for origin in ["https://evil.example", "http://127.0.0.1", "null"] {
            assert_eq!(raw(&h.serve, &[("authorization", &bearer), ("origin", origin)]).await, reqwest::StatusCode::FORBIDDEN, "{origin}");
            assert_eq!(raw(&h.serve, &[("origin", origin)]).await, reqwest::StatusCode::FORBIDDEN, "{origin}, no token");
        }
        assert_eq!(raw(&h.serve, &[("authorization", &bearer)]).await, reqwest::StatusCode::OK);
        h.serve.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A local agent not in a room is refused there, and nobody is asked.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_non_member_is_refused_with_no_card() {
        let dir = crate::store::tests::folder("door-serve-member");
        let h = open_in(&dir);
        let a = h.serve.add("Claude Code").unwrap();
        let c = client(&h.serve, &token_of(&h.serve, &a.id)).await;
        let room = board();
        for (tool, args) in [
            ("space_feed", json!({ "space": room })),
            ("space_call", json!({ "space": room, "tool": "show", "arguments": { "title": "hello" } })),
            ("table_read", json!({ "space": room, "path": "anything.txt" })),
        ] {
            let r = tokio::time::timeout(Duration::from_secs(10), result(&c, call(tool, args))).await.expect("answered without waiting");
            assert_eq!((r.is_error, words(&r)), (Some(true), crate::door::NOT_IN.to_owned()), "{tool}");
        }
        assert!(h.door.cards().is_empty());
        let _ = c.cancel().await;
        h.serve.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// After a restart: the same key, token, port and allowances.
    #[tokio::test(flavor = "multi_thread")]
    async fn after_a_restart_the_key_token_port_and_allowances_are_the_same() {
        let dir = crate::store::tests::folder("door-serve-restart");
        let (a, token, port, key, room) = {
            let h = open_in(&dir);
            let room = board();
            let a = h.serve.add("Claude Code").unwrap();
            admit(&h, &room, &a.id).await;
            h.door.set_allowance(&room, &a.slot, Reach::Talk, 2);
            let key = h.identity.agent_key(&AgentId::Local(a.id.clone()), &room).unwrap();
            let port = h.serve.lock().port.unwrap();
            h.serve.stop().await;
            let token = token_of(&h.serve, &a.id);
            (a, token, port, key, room)
        };
        let h = open_in(&dir);
        assert_eq!(h.serve.status(), DoorStatusView { port: Some(port), state: DoorState::Listening }, "listening again on the same port");
        assert_eq!(h.identity.agent_key(&AgentId::Local(a.id.clone()), &room), Some(key.clone()), "the same key");
        assert_eq!(token_of(&h.serve, &a.id), token, "the same token");
        assert_eq!(h.door.allowance(&room, &a.slot).per_day.get(&Reach::Talk), Some(&2), "the same allowance");
        assert_eq!(h.serve.list(), vec![a.clone()], "and the same line");
        // The old token works, on the allowance, with no card.
        let c = client(&h.serve, &token).await;
        let r = tokio::time::timeout(Duration::from_secs(10), result(&c, call("space_call", json!({ "space": room, "tool": "show", "arguments": { "title": "after a restart" } })))).await.expect("no card");
        assert_ne!(r.is_error, Some(true), "{}", words(&r));
        assert_eq!(moves(&h, &room).await.into_iter().find(|m| m["title"] == "after a restart").map(|m| m["by"].clone()), Some(json!(key)));
        let _ = c.cancel().await;
        h.serve.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Something else holding the door's port: the door says so, and never moves.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_port_taken_is_said_and_never_moved() {
        let dir = crate::store::tests::folder("door-serve-port");
        let (port, token) = {
            let h = open_in(&dir);
            let a = h.serve.add("Claude Code").unwrap();
            h.serve.stop().await;
            (h.serve.lock().port.unwrap(), token_of(&h.serve, &a.id))
        };
        let squatter = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
        let h = open_in(&dir);
        assert_eq!(h.serve.status(), DoorStatusView { port: Some(port), state: DoorState::PortTaken });
        assert!(h.serve.list()[0].connect_line.as_deref().unwrap().contains(&format!(":{port}/mcp")), "the line keeps its port");
        drop(squatter);
        assert_eq!(h.serve.status(), DoorStatusView { port: Some(port), state: DoorState::Listening }, "the same port, once it's free");
        assert_eq!(raw(&h.serve, &[("authorization", &format!("Bearer {token}"))]).await, reqwest::StatusCode::OK);
        h.serve.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The connect line, run by a shell as you'd paste it, hands Claude Code a
    /// helper that prints valid header JSON with the token; the line itself
    /// never holds the token. In a folder whose path has a space and a quote.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_line_never_holds_the_token_and_its_helper_prints_the_header() {
        let root = crate::store::tests::folder("door-serve-line");
        let dir = root.join("Application Support").join("it's $here");
        std::fs::create_dir_all(&dir).unwrap();
        let h = open_in(&dir);
        let a = h.serve.add("Claude Code").unwrap();
        let token = token_of(&h.serve, &a.id);
        let line = a.connect_line.clone().unwrap();
        assert!(!line.contains(&token) && !line.contains(&token[..16]), "the line never holds the token's bytes");
        assert!(line.starts_with(&format!("claude mcp add-json diverge-{} '", a.id)) && line.ends_with("' --scope user"));
        // A stand-in for `claude` that writes down what it was handed.
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let got = root.join("got.json");
        std::fs::write(bin.join("claude"), format!("#!/bin/sh\nprintf '%s' \"$4\" > {}\n", sh_single(&got))).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(bin.join("claude"), std::fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(std::fs::metadata(h.serve.token_file(&a.id)).unwrap().permissions().mode() & 0o777, 0o600, "the token is owner-only");
            assert_eq!(std::fs::metadata(h.serve.helper_file(&a.id)).unwrap().permissions().mode() & 0o777, 0o700, "the helper too");
            assert_eq!(std::fs::metadata(dir.join(DOOR_AGENTS_FILE)).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
        let ran = std::process::Command::new("sh").arg("-c").arg(&line).env("PATH", &path).status().unwrap();
        assert!(ran.success());
        let config: Value = serde_json::from_str(&std::fs::read_to_string(&got).unwrap()).unwrap();
        assert_eq!(config["type"], "http");
        assert_eq!(config["url"], url(&h.serve));
        assert_eq!(config["timeout"], CALL_TIMEOUT_MS);
        // Claude Code runs the helper in a shell and reads a JSON object of headers.
        let out = std::process::Command::new("sh").arg("-c").arg(config["headersHelper"].as_str().unwrap()).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let headers: serde_json::Map<String, Value> = serde_json::from_slice(&out.stdout).expect("valid header JSON");
        assert_eq!(headers.get("Authorization"), Some(&json!(format!("Bearer {token}"))));
        assert!(headers.values().all(Value::is_string));
        // And the door answers what the helper prints.
        let bearer = headers["Authorization"].as_str().unwrap().to_owned();
        assert_eq!(raw(&h.serve, &[("authorization", &bearer)]).await, reqwest::StatusCode::OK);
        // The door's own file keeps only a digest.
        let kept = std::fs::read_to_string(dir.join(DOOR_AGENTS_FILE)).unwrap();
        assert!(!kept.contains(&token) && kept.contains(&hex::encode(digest(&token))));
        h.serve.stop().await;
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A session is its opener's: another agent's token can't ride on it.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_session_is_its_openers_alone() {
        let dir = crate::store::tests::folder("door-serve-session");
        let h = open_in(&dir);
        let a = h.serve.add("Claude Code").unwrap();
        let b = h.serve.add("Another helper").unwrap();
        let http = reqwest::Client::new();
        let opened = http
            .post(url(&h.serve))
            .bearer_auth(token_of(&h.serve, &a.id))
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .body(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "a", "version": "1" } } }).to_string())
            .send()
            .await
            .unwrap();
        let session = opened.headers().get("mcp-session-id").expect("a session").to_str().unwrap().to_owned();
        let riding = http
            .post(url(&h.serve))
            .bearer_auth(token_of(&h.serve, &b.id))
            .header("mcp-session-id", &session)
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .body(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(riding.status(), reqwest::StatusCode::NOT_FOUND);
        h.serve.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Measured: what the door does when a client stops waiting on a card.
    /// A client that sends `notifications/cancelled` gets its card withdrawn,
    /// and a late yes does nothing. A client that only drops the request,
    /// saying nothing, leaves the card up: the door can't tell, until the
    /// client ends its session.
    #[tokio::test(flavor = "multi_thread")]
    async fn cancelling_a_request_withdraws_its_card_and_dropping_one_silently_does_not() {
        let dir = crate::store::tests::folder("door-serve-cancel");
        let h = open_in(&dir);
        let room = board();
        let a = h.serve.add("Claude Code").unwrap();
        admit(&h, &room, &a.id).await;
        let mut events = h.door.watch(CancellationToken::new());
        let c = client(&h.serve, &token_of(&h.serve, &a.id)).await;
        let request = |title: &str| ClientRequest::CallToolRequest(rmcp::model::CallToolRequest::new(call("space_call", json!({ "space": room, "tool": "show", "arguments": { "title": title } }))));

        let handle = c.peer().send_cancellable_request(request("cancelled"), PeerRequestOptions::no_options()).await.unwrap();
        let card = next_card(&mut events).await;
        handle.cancel(Some("stopped waiting".into())).await.unwrap();
        match tokio::time::timeout(Duration::from_secs(10), events.next()).await.expect("an event") {
            Some(CardEvent::Withdrawn { id }) => assert_eq!(id, card.id),
            other => panic!("expected the card withdrawn, got {other:?}"),
        }
        assert_eq!(h.door.answer(card.id, YES.into()).unwrap_err(), crate::door::WITHDRAWN, "a late yes does nothing");

        let dropped = {
            let peer = c.peer().clone();
            let req = request("dropped");
            tokio::spawn(async move { peer.send_request(req).await })
        };
        let card = next_card(&mut events).await;
        dropped.abort();
        let _ = dropped.await;
        assert!(tokio::time::timeout(Duration::from_millis(1500), events.next()).await.is_err(), "nothing tells the door, so the card stays");
        assert!(h.door.cards().iter().any(|c| c.id == card.id));
        // Ending the session does tell it: what it still waited on is withdrawn.
        let _ = c.cancel().await;
        match tokio::time::timeout(Duration::from_secs(10), events.next()).await.expect("an event") {
            Some(CardEvent::Withdrawn { id }) => assert_eq!(id, card.id),
            other => panic!("expected the card withdrawn when the session ended, got {other:?}"),
        }
        h.serve.stop().await;
        assert!(!moves(&h, &room).await.iter().any(|m| m["title"] == "cancelled"), "nothing was said");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
