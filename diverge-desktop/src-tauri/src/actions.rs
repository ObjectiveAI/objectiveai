//! The action registry: everything a person can do in this app, in one
//! place. The page calls these; an agent's door (MCP, after draft one)
//! will call the same functions. No action lives only in a click handler.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use diverge_sdk::daemon::endpoints::agents;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::volumes as pv;

use crate::catalog;
use crate::daemon::{Daemon, NewProvider, ProviderEntry};
use crate::machines::{Machines, ReadFrame};
use crate::door::Door;
use crate::spaces::{self, Spaces};
use crate::tabs::Tabs;
use crate::view::*;

pub struct AppState {
    pub daemon: Arc<dyn Daemon>,
    /// Each machine's own volumes: the provider protocol's verbs.
    pub machines: Arc<dyn Machines>,
    pub spaces: Arc<dyn Spaces>,
    pub door: Arc<Door>,
    pub stand_in_host: Option<PathBuf>,
    pub scopes: Mutex<HashMap<String, CancellationToken>>,
    pub next_scope: AtomicU64,
    pub tabs: Mutex<Tabs>,
    pub views: Mutex<Vec<SavedView>>,
    pub views_file: PathBuf,
    /// Machine names, yours: identity key → name.
    pub machine_names: Mutex<HashMap<String, String>>,
    pub machine_names_file: PathBuf,
    /// What this app last stated each agent mounts: agent name → mounts.
    pub agent_mounts: Mutex<HashMap<String, AgentMounts>>,
    pub agent_mounts_file: PathBuf,
}

impl AppState {
    fn remember_mounts(&self, name: &str, mounts: Option<AgentMounts>) {
        let mut all = self.agent_mounts.lock().unwrap();
        match mounts {
            Some(m) => all.insert(name.to_owned(), m),
            None => all.remove(name),
        };
        if let Ok(json) = serde_json::to_string_pretty(&*all) {
            let _ = std::fs::write(&self.agent_mounts_file, json);
        }
    }

    fn open_scope(&self, prefix: &str) -> (String, CancellationToken) {
        let id = format!("{prefix}-{}", self.next_scope.fetch_add(1, Ordering::Relaxed));
        let token = CancellationToken::new();
        self.scopes.lock().unwrap().insert(id.clone(), token.clone());
        (id, token)
    }

    fn close_scope(&self, id: &str) -> bool {
        match self.scopes.lock().unwrap().remove(id) {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    fn save_views(&self, views: &[SavedView]) {
        if let Ok(json) = serde_json::to_string_pretty(views) {
            let _ = std::fs::write(&self.views_file, json);
        }
    }
}

/// The registry, by name. Keep in step with `handlers()` in main.rs.
pub const REGISTRY: &[(&str, &str)] = &[
    ("agents_list", "List every agent"),
    ("agents_create", "Make an agent from an image, settings, limits and mounts"),
    ("agents_delete", "Remove an agent (the daemon refuses while it is running)"),
    ("agents_edit", "Change what an agent mounts (the daemon refuses while it is running)"),
    ("agents_mounts", "What this app last stated an agent mounts"),
    ("agents_message", "Send an agent a message; resolves when delivered"),
    ("agents_message_take_back", "Take back a message that has not been delivered yet"),
    ("logs_open", "Read an agent's log — filtered, reshaped by jq, perhaps watched"),
    ("scope_close", "Stop watching something"),
    ("volumes_list", "List a machine's storage"),
    ("volumes_stat", "How full a volume is"),
    ("volumes_tree", "What a volume holds (a snapshot)"),
    ("volumes_read", "Open a file in a volume"),
    ("volumes_write", "Save a file in a volume"),
    ("volumes_room", "How much room there is for a new volume"),
    ("volumes_create", "Make a volume"),
    ("volumes_room_for", "How far a volume may grow"),
    ("volumes_edit", "Resize a volume, or change what becomes of what agents write"),
    ("volumes_delete", "Delete a volume (refused while anything holds it)"),
    ("home_feed", "Everything that happened across your Spaces, newest first"),
    ("people_list", "Everyone you share a Space with"),
    ("profile_get", "You: receipts, what you've shown, your agents, machines and storage"),
    ("spaces_home", "Your own Home Space"),
    ("spaces_list", "The Spaces you host or have joined"),
    ("spaces_get", "A Space: its charter, members and verbs"),
    ("spaces_feed", "What has happened in a Space"),
    ("spaces_call", "Do something in a Space: one of its verbs, with arguments"),
    ("spaces_watch", "Hear when a Space changes"),
    ("spaces_host", "Host a Space on your machine"),
    ("spaces_join", "Join a Space with an invite"),
    ("spaces_leave", "Leave a Space (or end one you host)"),
    ("spaces_invite", "The invite to hand a friend"),
    ("knocks_watch", "Hear who is at the door of Spaces you host"),
    ("knocks_answer", "Let someone in, or not"),
    ("cards_watch", "Hear when an agent asks you something"),
    ("cards_answer", "Answer an agent's card"),
    ("door_tools", "What agents can do through the app (the agent door)"),
    ("machines_list", "List the machines the daemon can run on"),
    ("machines_add", "Add a machine: one you dial, or one that dials you"),
    ("machines_remove", "Remove a machine"),
    ("machines_rename", "Call a machine by a name of your own"),
    ("machines_names", "The names you gave your machines"),
    ("views_list", "List saved Views"),
    ("views_save", "Save a View (a saved log request)"),
    ("views_delete", "Delete a saved View"),
    ("catalog_images", "The six agent images and their settings"),
    ("catalog_check", "Whether settings are ones an image accepts"),
    ("tabs_snapshot", "What is open"),
    ("tabs_open", "Open (or focus) a tab"),
    ("tabs_close", "Close a tab"),
    ("tabs_focus", "Focus a tab"),
    ("actions_list", "This list"),
    ("app_info", "Whether the daemon is the stand-in, and the contract pin"),
];

#[tauri::command]
pub fn actions_list() -> Vec<ActionInfo> {
    REGISTRY.iter().map(|(name, does)| ActionInfo { name: (*name).into(), does: (*does).into() }).collect()
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        stand_in: state.stand_in_host.is_some(),
        contract_pin: include_str!("../../CONTRACT_PIN").lines().next().unwrap_or_default().to_owned(),
        stand_in_host: state.stand_in_host.as_ref().map(|p| p.display().to_string()),
    }
}

#[tauri::command]
pub fn catalog_images() -> Vec<ImageKindView> {
    catalog::ALL
        .into_iter()
        .map(|kind| ImageKindView { key: kind.key().into(), image_name: kind.image_name(), digest: catalog::UNBUILT_DIGEST.into(), schema: kind.schema() })
        .collect()
}

/// Checked with the image's own types, from Ronald's source.
#[tauri::command]
pub fn catalog_check(kind: String, arguments: serde_json::Value) -> Result<(), String> {
    let kind = catalog::ALL.into_iter().find(|k| k.key() == kind).ok_or_else(|| format!("no image called {kind}"))?;
    kind.check(&arguments)
}

// --- agents --------------------------------------------------------------

#[tauri::command]
pub async fn agents_list(state: State<'_, AppState>) -> Result<AgentsListed, String> {
    let frames = state.daemon.agents_list(agents::list::client::request::Frame {}).collect::<Vec<_>>().await;
    Ok(listed(frames))
}

#[tauri::command]
pub async fn agents_create(state: State<'_, AppState>, input: CreateAgentInput) -> Result<CreateOutcome, String> {
    let request = input.into_request()?;
    let name = request.name.clone();
    let mounts = AgentMounts::of_create(&request);
    let outcome: CreateOutcome = state.daemon.agents_create(request).await.into();
    if matches!(outcome, CreateOutcome::Created) {
        state.remember_mounts(&name, Some(mounts));
    }
    // A new agent of yours is a member of your Home, so it can report there.
    if matches!(outcome, CreateOutcome::Created) {
        if let Some(home) = state.spaces.home().await {
            let params = rmcp::model::CallToolRequestParams::new("admit").with_arguments(serde_json::json!({ "name": name, "agent": true }).as_object().cloned().unwrap_or_default());
            let _ = state.spaces.call(&home, params, spaces::Caller::Person).await;
        }
    }
    Ok(outcome)
}

#[tauri::command]
pub async fn agents_edit(state: State<'_, AppState>, input: EditMountsInput) -> Result<EditOutcome, String> {
    let request = input.into_request();
    let Some(before) = state.agent_mounts.lock().unwrap().get(&request.name).cloned() else {
        return Err("this agent was made outside this app, so its mounts are not known here".into());
    };
    let after = before.edited(&request);
    let name = request.name.clone();
    let outcome: EditOutcome = state.daemon.agents_edit(request).await.into();
    if matches!(outcome, EditOutcome::Edited) {
        state.remember_mounts(&name, Some(after));
    }
    Ok(outcome)
}

/// What this app last stated the agent mounts; none if it was made elsewhere.
#[tauri::command]
pub fn agents_mounts(state: State<'_, AppState>, name: String) -> Option<MountsView> {
    state.agent_mounts.lock().unwrap().get(&name).map(AgentMounts::view)
}

#[tauri::command]
pub async fn agents_delete(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<DeleteOutcome, String> {
    let outcome: DeleteOutcome = state.daemon.agents_delete(agents::delete::client::request::Frame { name: name.clone() }).await.into();
    if matches!(outcome, DeleteOutcome::Deleted) {
        state.remember_mounts(&name, None);
        let snapshot = state.tabs.lock().unwrap().close(&crate::tabs::key_of(&TabKind::Agent { name }));
        let _ = app.emit("tabs://changed", snapshot);
    }
    Ok(outcome)
}

#[tauri::command]
pub async fn agents_message(state: State<'_, AppState>, name: String, text: String, ticket: String) -> Result<MessageOutcome, String> {
    let token = CancellationToken::new();
    state.scopes.lock().unwrap().insert(ticket.clone(), token.clone());
    let content = vec![serde_json::from_value(serde_json::json!({ "type": "text", "text": text })).map_err(|e| e.to_string())?];
    let outcome = state.daemon.agents_message(agents::message::client::request::Frame { name, content }, token).await;
    state.scopes.lock().unwrap().remove(&ticket);
    Ok(outcome.into())
}

#[tauri::command]
pub fn agents_message_take_back(state: State<'_, AppState>, ticket: String) -> bool {
    state.close_scope(&ticket)
}

#[tauri::command]
pub fn logs_open(state: State<'_, AppState>, query: LogsQuery, on_event: Channel<LogEvent>) -> Result<String, String> {
    let request = query.into_request()?;
    let (id, token) = state.open_scope("logs");
    let mut frames = state.daemon.agents_logs(request, token.clone());
    tauri::async_runtime::spawn(async move {
        while let Some(frame) = frames.next().await {
            if on_event.send(LogEvent::from(frame)).is_err() {
                token.cancel();
                return;
            }
        }
        let _ = on_event.send(LogEvent::End);
    });
    Ok(id)
}

#[tauri::command]
pub fn scope_close(state: State<'_, AppState>, id: String) -> bool {
    state.close_scope(&id)
}

// --- storage (each machine's own volumes) ---------------------------------

#[tauri::command]
pub async fn volumes_list(state: State<'_, AppState>, machine: ProviderView) -> Result<VolumesListed, String> {
    Ok(state.machines.volumes_list(&(&machine).into()).await.into())
}

#[tauri::command]
pub async fn volumes_stat(state: State<'_, AppState>, machine: ProviderView, name: String) -> Result<VolumeStat, String> {
    Ok(state.machines.volumes_stat(&(&machine).into(), pv::stat::client::request::Frame { name }).await.into())
}

#[tauri::command]
pub async fn volumes_tree(state: State<'_, AppState>, machine: ProviderView, name: String) -> Result<VolumeTree, String> {
    Ok(state.machines.volumes_filetree(&(&machine).into(), pv::filetree::client::request::Frame { name, path: Vec::new() }).await.into())
}

fn path_components(path: &str) -> Vec<String> {
    path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect()
}

#[tauri::command]
pub async fn volumes_read(state: State<'_, AppState>, machine: ProviderView, name: String, path: String) -> Result<FileRead, String> {
    let mut frames = state.machines.volumes_read(&(&machine).into(), pv::read::client::request::Frame { name, path: path_components(&path) });
    let mut bytes = Vec::new();
    while let Some(frame) = frames.next().await {
        match frame {
            ReadFrame::Body(body) => bytes.extend_from_slice(&body),
            ReadFrame::Error(error) => return Ok(FileRead::Error { message: error_text(&error) }),
        }
    }
    let size = bytes.len() as u64;
    Ok(match String::from_utf8(bytes) {
        Ok(text) => FileRead::Text { text, bytes: size },
        Err(_) => FileRead::Binary { bytes: size },
    })
}

#[tauri::command]
pub async fn volumes_write(state: State<'_, AppState>, machine: ProviderView, name: String, path: String, text: String) -> Result<FileWritten, String> {
    Ok(state
        .machines
        .volumes_write(&(&machine).into(), pv::write::client::request::Frame { name, path: path_components(&path) }, text.into_bytes())
        .await
        .into())
}

#[tauri::command]
pub async fn volumes_room(state: State<'_, AppState>, machine: ProviderView) -> Result<Capacity, String> {
    Ok(state.machines.volumes_create_capacity(&(&machine).into()).await.into())
}

#[tauri::command]
pub async fn volumes_create(state: State<'_, AppState>, machine: ProviderView, name: String, bytes: u64, mode: VolumeMode) -> Result<VolumeChange, String> {
    Ok(state.machines.volumes_create(&(&machine).into(), pv::create::client::request::Frame { name, bytes, mode: mode.into() }).await.into())
}

#[tauri::command]
pub async fn volumes_room_for(state: State<'_, AppState>, machine: ProviderView, name: String) -> Result<Capacity, String> {
    Ok(state.machines.volumes_edit_capacity(&(&machine).into(), pv::edit_capacity::client::request::Frame { name }).await.into())
}

#[tauri::command]
pub async fn volumes_edit(state: State<'_, AppState>, machine: ProviderView, name: String, bytes: Option<u64>, mode: Option<VolumeMode>) -> Result<VolumeChange, String> {
    use pv::edit::client::request::Change;
    let change = match (bytes, mode) {
        (Some(bytes), Some(mode)) => Change::Both { bytes, mode: mode.into() },
        (Some(bytes), None) => Change::Bytes(bytes),
        (None, Some(mode)) => Change::Mode(mode.into()),
        (None, None) => return Err("nothing to change".into()),
    };
    Ok(state.machines.volumes_edit(&(&machine).into(), pv::edit::client::request::Frame { name, change }).await.into())
}

#[tauri::command]
pub async fn volumes_delete(state: State<'_, AppState>, machine: ProviderView, name: String) -> Result<VolumeChange, String> {
    Ok(state.machines.volumes_delete(&(&machine).into(), pv::delete::client::request::Frame { name }).await.into())
}

// --- Spaces ----------------------------------------------------------------

fn space_id(id: &str) -> spaces::Id {
    spaces::Id { id: id.to_owned() }
}

/// An invite as text: `diverge://space/<id>?host=<kind>:<value>&key=<what to present>`.
pub fn invite_text(invite: &spaces::Invite) -> String {
    use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
    let host = match &invite.host {
        Identity::Outgoing { address } => format!("outgoing:{address}"),
        Identity::IncomingUnbrokered { identity } => format!("incoming:{identity}"),
    };
    format!("diverge://space/{}?host={}&key={}", invite.connect.id, host.replace(' ', "%20"), invite.connect.authorization.replace(' ', "%20"))
}

fn parse_invite(text: &str) -> Result<spaces::Invite, String> {
    use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
    let rest = text.trim().strip_prefix("diverge://space/").ok_or("that isn't an invite")?;
    let (id, query) = rest.split_once('?').ok_or("the invite is missing its host and key")?;
    let mut host = None;
    let mut key = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').ok_or("the invite is malformed")?;
        let v = v.replace("%20", " ");
        match k {
            "host" => host = Some(v),
            "key" => key = Some(v),
            _ => {}
        }
    }
    let host = host.ok_or("the invite names no host")?;
    let host = match host.split_once(':') {
        Some(("outgoing", address)) => Identity::Outgoing { address: address.into() },
        Some(("incoming", identity)) => Identity::IncomingUnbrokered { identity: identity.into() },
        _ => return Err("the invite's host is not one the daemon can name".into()),
    };
    Ok(spaces::Invite { host, connect: spaces::Connect { id: id.into(), authorization: key.ok_or("the invite has no key")? } })
}

#[tauri::command]
pub async fn spaces_home(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.spaces.home().await.map(|id| id.id))
}

#[tauri::command]
pub async fn home_feed(state: State<'_, AppState>) -> Result<Vec<HomeMove>, String> {
    let mut out = Vec::new();
    for e in state.spaces.list().await {
        let summary: SpaceSummary = (&e).into();
        if let Ok(r) = state.spaces.read(&e.id, spaces::stub::rooms::FEED).await {
            let moves = r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => serde_json::from_str::<Vec<MoveView>>(text).ok(), _ => None }).next().unwrap_or_default();
            out.extend(moves.into_iter().map(|entry| HomeMove { space: summary.clone(), entry }));
        }
    }
    out.sort_by(|a, b| b.entry.at.cmp(&a.entry.at));
    out.truncate(300);
    Ok(out)
}

#[tauri::command]
pub async fn profile_get(state: State<'_, AppState>) -> Result<ProfileView, String> {
    let agents = listed(state.daemon.agents_list(agents::list::client::request::Frame {}).collect::<Vec<_>>().await).agents;
    let mut mine: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
    let home = state.spaces.home().await.map(|id| id.id);
    let mut receipts = Vec::new();
    let mut shows = Vec::new();
    for e in state.spaces.list().await {
        mine.push(e.joined_as.clone());
        let summary: SpaceSummary = (&e).into();
        if let Ok(r) = state.spaces.read(&e.id, spaces::stub::rooms::FEED).await {
            let moves = r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => serde_json::from_str::<Vec<MoveView>>(text).ok(), _ => None }).next().unwrap_or_default();
            for m in moves {
                let to = m.fields.get("to").and_then(serde_json::Value::as_str).unwrap_or_default().to_owned();
                if m.kind == "receipt" && mine.contains(&to) {
                    let for_title = m.body.strip_prefix("Completed: ").unwrap_or(&m.body).to_owned();
                    receipts.push(ReceiptView { title: m.title.clone(), for_title, space: summary.clone(), to, at: m.at.clone() });
                } else if m.kind == "show" && Some(&e.id.id) == home.as_ref() && m.author == e.joined_as {
                    shows.push(m);
                }
            }
        }
    }
    receipts.sort_by(|a, b| b.at.cmp(&a.at));
    shows.sort_by(|a, b| b.at.cmp(&a.at));
    let machines = machines_with_volumes(&state).await;
    let volumes: Vec<VolumeView> = machines.iter().flat_map(|m| m.volumes.clone()).collect();
    Ok(ProfileView { receipts, shows, agents, machines, volumes, home })
}

#[tauri::command]
pub async fn people_list(state: State<'_, AppState>) -> Result<Vec<PersonView>, String> {
    let mut people: std::collections::BTreeMap<String, PersonView> = std::collections::BTreeMap::new();
    for e in state.spaces.list().await {
        let me = e.joined_as.clone();
        if let Ok(r) = state.spaces.read(&e.id, spaces::stub::rooms::MEMBERS).await {
            let members = r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => serde_json::from_str::<Vec<MemberView>>(text).ok(), _ => None }).next().unwrap_or_default();
            for m in members {
                if m.name == me || m.name == "me" {
                    continue;
                }
                let p = people.entry(m.name.clone()).or_insert(PersonView { name: m.name.clone(), is_agent: m.is_agent, spaces: Vec::new() });
                p.spaces.push(e.id.id.clone());
            }
        }
    }
    Ok(people.into_values().collect())
}

#[tauri::command]
pub async fn spaces_list(state: State<'_, AppState>) -> Result<Vec<SpaceSummary>, String> {
    Ok(state.spaces.list().await.iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn spaces_get(state: State<'_, AppState>, id: String) -> Result<SpaceView, String> {
    let entries = state.spaces.list().await;
    let entry = entries.iter().find(|e| e.id.id == id).ok_or("no such Space")?;
    let sid = space_id(&id);
    let tools = state.spaces.tools(&sid).await.map_err(|e| e.message.to_string())?;
    let members = match state.spaces.read(&sid, spaces::stub::rooms::MEMBERS).await {
        Ok(r) => r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => serde_json::from_str::<Vec<MemberView>>(text).ok(), _ => None }).next().unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let charter = match state.spaces.read(&sid, spaces::stub::rooms::CHARTER).await {
        Ok(r) => r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()), _ => None }).next().unwrap_or_default(),
        Err(_) => String::new(),
    };
    Ok(SpaceView { summary: entry.into(), charter, members, tools: tools.tools.iter().map(Into::into).collect() })
}

#[tauri::command]
pub async fn spaces_feed(state: State<'_, AppState>, id: String) -> Result<FeedRead, String> {
    Ok(match state.spaces.read(&space_id(&id), spaces::stub::rooms::FEED).await {
        Ok(r) => {
            let moves = r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => serde_json::from_str::<Vec<MoveView>>(text).ok(), _ => None }).next().unwrap_or_default();
            FeedRead::Feed { moves }
        }
        Err(e) => FeedRead::Error { message: e.message.to_string() },
    })
}

#[tauri::command]
pub async fn spaces_call(state: State<'_, AppState>, id: String, tool: String, arguments: serde_json::Value) -> Result<CallOutcome, String> {
    let params = rmcp::model::CallToolRequestParams::new(tool).with_arguments(arguments.as_object().cloned().unwrap_or_default());
    Ok(match state.spaces.call(&space_id(&id), params, spaces::Caller::Person).await {
        Ok(result) => {
            let text = result.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n");
            if result.is_error == Some(true) { CallOutcome::Error { message: text } } else { CallOutcome::Ok { text } }
        }
        Err(e) => CallOutcome::Error { message: e.message.to_string() },
    })
}

#[tauri::command]
pub fn spaces_watch(state: State<'_, AppState>, id: String, on_event: Channel<SpaceEvent>) -> String {
    let (scope, token) = state.open_scope("space");
    let mut frames = state.spaces.notifications(&space_id(&id), token.clone());
    tauri::async_runtime::spawn(async move {
        while let Some(n) = frames.next().await {
            if on_event.send(SpaceEvent::from(n)).is_err() {
                token.cancel();
                return;
            }
        }
        let _ = on_event.send(SpaceEvent::End);
    });
    scope
}

#[tauri::command]
pub async fn spaces_host(state: State<'_, AppState>, input: HostSpaceInput) -> Result<HostOutcome, String> {
    use diverge_sdk::shared::containers::request::{Container, Image};
    let container = Container {
        image: Image { name: format!("diverge-space-{}", input.kind), digest: catalog::UNBUILT_DIGEST.into() },
        memory: 1 << 30,
        disk: 1 << 30,
        volume_mounts: Vec::new(),
        fuse_file_mounts: Vec::new(),
        fuse_directory_mounts: Vec::new(),
        arguments: serde_json::json!({ "title": input.title, "kind": input.kind, "charter": input.charter, "invite": input.invite }),
    };
    Ok(match state.spaces.host(container).await {
        Ok(id) => HostOutcome::Hosted { id: id.id },
        Err(e) => HostOutcome::Error { message: error_text(&e) },
    })
}

#[tauri::command]
pub async fn spaces_join(state: State<'_, AppState>, invite: String, as_name: String) -> Result<JoinOutcome, String> {
    let invite = parse_invite(&invite)?;
    Ok(state.spaces.join(invite, as_name).await.into())
}

#[tauri::command]
pub async fn spaces_leave(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.spaces.leave(&space_id(&id)).await?;
    let snapshot = state.tabs.lock().unwrap().close(&crate::tabs::key_of(&TabKind::Space { id }));
    let _ = app.emit("tabs://changed", snapshot);
    Ok(())
}

#[tauri::command]
pub async fn spaces_invite(state: State<'_, AppState>, id: String) -> Result<Option<InviteView>, String> {
    Ok(state.spaces.invite(&space_id(&id)).await.map(|i| InviteView { text: invite_text(&i) }))
}

#[tauri::command]
pub fn knocks_watch(state: State<'_, AppState>, on_event: Channel<KnockEvent>) -> String {
    let (scope, token) = state.open_scope("knocks");
    let spaces = state.spaces.clone();
    let mut frames = state.spaces.knocks(token.clone());
    tauri::async_runtime::spawn(async move {
        while let Some(k) = frames.next().await {
            let title = spaces.list().await.into_iter().find(|e| e.id == k.space).map(|e| e.title).unwrap_or_else(|| k.space.id.clone());
            let view = KnockView { knock_id: k.knock_id, space: k.space.id.clone(), space_title: title, address: k.authorize.address.to_string(), authorization: k.authorize.authorization.clone(), at: k.at.to_rfc3339() };
            if on_event.send(KnockEvent::Knock { knock: view }).is_err() {
                token.cancel();
                return;
            }
        }
        let _ = on_event.send(KnockEvent::End);
    });
    scope
}

#[tauri::command]
pub async fn knocks_answer(state: State<'_, AppState>, knock_id: u64, yes: bool) -> Result<(), String> {
    state.spaces.answer(knock_id, if yes { spaces::Answer::Authorized } else { spaces::Answer::Denied }).await
}

// --- cards: the agent door's questions ------------------------------------

#[tauri::command]
pub fn cards_watch(state: State<'_, AppState>, on_event: Channel<CardEvent>) -> String {
    let (scope, token) = state.open_scope("cards");
    let mut frames = state.door.watch(token.clone());
    tauri::async_runtime::spawn(async move {
        while let Some(e) = frames.next().await {
            if on_event.send(e).is_err() {
                token.cancel();
                return;
            }
        }
        let _ = on_event.send(CardEvent::End);
    });
    scope
}

#[tauri::command]
pub fn cards_answer(state: State<'_, AppState>, id: u64, answer: String) -> Result<(), String> {
    state.door.answer(id, answer)
}

#[tauri::command]
pub fn door_tools(state: State<'_, AppState>) -> Vec<ToolView> {
    state.door.tools().tools.iter().map(Into::into).collect()
}

// --- machines (ours until the wire has them) -----------------------------

fn machine(entry: ProviderEntry) -> MachineView {
    MachineView { identity: (&entry.identity).into(), volumes: Vec::new(), volumes_problem: None, added: entry.added.to_rfc3339(), name: None }
}

/// Every machine, named, with its own listing of its volumes.
async fn machines_with_volumes(state: &AppState) -> Vec<MachineView> {
    let mut out = Vec::new();
    for entry in state.daemon.providers_list().await {
        let on: Identity = entry.identity.clone();
        let mut m = named(state, machine(entry));
        match VolumesListed::from(state.machines.volumes_list(&on).await) {
            VolumesListed::Volumes { volumes } => m.volumes = volumes,
            VolumesListed::Error { message } => m.volumes_problem = Some(message),
        }
        out.push(m);
    }
    out
}

fn named(state: &AppState, mut m: MachineView) -> MachineView {
    m.name = state.machine_names.lock().unwrap().get(&identity_key(&m.identity)).cloned();
    m
}

#[tauri::command]
pub async fn machines_list(state: State<'_, AppState>) -> Result<Vec<MachineView>, String> {
    Ok(machines_with_volumes(&state).await)
}

#[tauri::command]
pub fn machines_names(state: State<'_, AppState>) -> HashMap<String, String> {
    state.machine_names.lock().unwrap().clone()
}

#[tauri::command]
pub fn machines_rename(state: State<'_, AppState>, identity: ProviderView, name: String) -> HashMap<String, String> {
    let mut names = state.machine_names.lock().unwrap();
    let key = identity_key(&identity);
    if name.trim().is_empty() {
        names.remove(&key);
    } else {
        names.insert(key, name.trim().to_owned());
    }
    if let Ok(json) = serde_json::to_string_pretty(&*names) {
        let _ = std::fs::write(&state.machine_names_file, json);
    }
    names.clone()
}

#[tauri::command]
pub async fn machines_add(state: State<'_, AppState>, input: NewMachineInput) -> Result<MachineView, String> {
    let provider = match input {
        NewMachineInput::Dial { address, key } => NewProvider::Dial { address, key },
        NewMachineInput::Accept { identity, key } => NewProvider::Accept { identity, key },
    };
    state.daemon.providers_add(provider).await.map(machine)
}

#[tauri::command]
pub async fn machines_remove(state: State<'_, AppState>, identity: ProviderView) -> Result<(), String> {
    state.daemon.providers_remove((&identity).into()).await
}

// --- saved Views -----------------------------------------------------------

#[tauri::command]
pub fn views_list(state: State<'_, AppState>) -> Vec<SavedView> {
    state.views.lock().unwrap().clone()
}

#[tauri::command]
pub fn views_save(state: State<'_, AppState>, mut view: SavedView) -> Result<SavedView, String> {
    view.query.clone().into_request()?;
    if view.title.trim().is_empty() {
        return Err("a View needs a title".into());
    }
    if view.id.is_empty() {
        view.id = format!("view-{}", chrono::Utc::now().timestamp_millis());
    }
    view.saved = chrono::Utc::now().to_rfc3339();
    let mut views = state.views.lock().unwrap();
    match views.iter_mut().find(|v| v.id == view.id) {
        Some(existing) => *existing = view.clone(),
        None => views.push(view.clone()),
    }
    state.save_views(&views);
    Ok(view)
}

#[tauri::command]
pub fn views_delete(state: State<'_, AppState>, id: String) {
    let mut views = state.views.lock().unwrap();
    views.retain(|v| v.id != id);
    state.save_views(&views);
}

// --- tabs --------------------------------------------------------------------

#[tauri::command]
pub fn tabs_snapshot(state: State<'_, AppState>) -> TabsSnapshot {
    state.tabs.lock().unwrap().snapshot()
}

#[tauri::command]
pub fn tabs_open(app: AppHandle, state: State<'_, AppState>, tab: TabKind) -> TabsSnapshot {
    let snapshot = state.tabs.lock().unwrap().open(tab);
    let _ = app.emit("tabs://changed", snapshot.clone());
    snapshot
}

#[tauri::command]
pub fn tabs_close(app: AppHandle, state: State<'_, AppState>, key: String) -> TabsSnapshot {
    let snapshot = state.tabs.lock().unwrap().close(&key);
    let _ = app.emit("tabs://changed", snapshot.clone());
    snapshot
}

#[tauri::command]
pub fn tabs_focus(app: AppHandle, state: State<'_, AppState>, key: String) -> TabsSnapshot {
    let snapshot = state.tabs.lock().unwrap().focus(&key);
    let _ = app.emit("tabs://changed", snapshot.clone());
    snapshot
}
