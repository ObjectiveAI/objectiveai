//! The action registry: everything a person can do in this app, in one
//! place. The page calls these; an agent's door (MCP, after draft one)
//! will call the same functions. No action lives only in a click handler.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use indexmap::IndexMap;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};
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
    /// Your keys: personas, your agents' keys, counters. Ours, not the wire's.
    pub identity: Arc<crate::identity::Identity>,
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
    /// Your copies of every room's record, one file each, kept as you go.
    pub records_dir: PathBuf,
    /// The last hash of each copy, so a copy is written only when it grew.
    pub record_heads: Mutex<HashMap<String, String>>,
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
    ("spaces_door", "Read an invite before knocking: the room's rules, its verbs, who hosts it"),
    ("spaces_join", "Knock with an invite, as your usual self or a fresh persona"),
    ("spaces_leave", "Leave a Space (or end one you host)"),
    ("spaces_invite", "The invite to hand a friend"),
    ("knocks_watch", "Hear who is at the door of Spaces you host"),
    ("knocks_answer", "Let someone in, or not"),
    ("asks_send", "Send one ask to several rooms at once, followed as one thread"),
    ("spaces_doorways", "The rooms a room vouches for"),
    ("vouch_for", "Vouch for someone: your seal on their key, to hand them"),
    ("spaces_admitted", "Everyone a room you host let in, listed or not"),
    ("spaces_restart", "Run a room you host again from its record"),
    ("spaces_continue", "Continue a room whose host is gone, from your copy of its record"),
    ("table_tree", "What's on a room's table"),
    ("table_read", "Open a file on a room's table"),
    ("table_write", "Put a file on a room's table"),
    ("table_transfer", "Move a file from one room's table to another's, on one machine"),
    ("personas_list", "Your personas, and the rooms you're each one in"),
    ("persona_rename", "Rename one of your personas"),
    ("allowance_get", "How many moves an agent may make in a room each day without asking"),
    ("allowance_set", "Let an agent make some moves in a room each day without asking"),
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
            let _ = admit_agent(&state, &home, &name).await;
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

async fn read_text(state: &AppState, id: &spaces::Id, uri: &str) -> Option<String> {
    let r = state.spaces.read(id, uri).await.ok()?;
    r.contents.iter().find_map(|c| match c {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text.clone()),
        _ => None,
    })
}

async fn read_json<T: serde::de::DeserializeOwned>(state: &AppState, id: &spaces::Id, uri: &str) -> Option<T> {
    serde_json::from_str(&read_text(state, id, uri).await?).ok()
}

fn text_of(result: &rmcp::model::CallToolResult) -> String {
    result.content.iter().filter_map(|c| c.as_text().map(|t| t.text.clone())).collect::<Vec<_>>().join("\n")
}

/// Your call to a room: sealed as whoever you are there.
async fn call_as_you(state: &AppState, id: &spaces::Id, tool: &str, arguments: serde_json::Value) -> Result<rmcp::model::CallToolResult, String> {
    let mut params = rmcp::model::CallToolRequestParams::new(tool.to_owned()).with_arguments(arguments.as_object().cloned().unwrap_or_default());
    state.identity.seal(&state.identity.you_in(&id.id), &id.id, &mut params)?;
    state.spaces.call(id, params).await.map_err(|e| e.message.to_string())
}

/// Let one of your agents into a room you host, tethered to who you are there.
pub async fn admit_agent(state: &AppState, room: &spaces::Id, agent: &str) -> Result<(), String> {
    let (key, tether) = state.identity.agent_in(agent, Some(&room.id));
    let person = state.identity.in_room(&room.id).unwrap_or_else(|| state.identity.usual());
    call_as_you(state, room, "admit", serde_json::json!({ "key": key, "name": agent, "is_agent": true, "agent_of": person.key, "tether": tether })).await.map(|_| ())
}

/// Keep your copy of a room's record, checked, whenever it has grown.
async fn keep_copy(state: &AppState, id: &spaces::Id) {
    let Some(record) = read_json::<serde_json::Value>(state, id, diverge_desktop_room::room::RECORD).await else { return };
    let moves: Vec<diverge_desktop_room::Move> = serde_json::from_value(record["moves"].clone()).unwrap_or_default();
    let head = moves.last().map(|m| m.hash.clone()).unwrap_or_default();
    if state.record_heads.lock().unwrap().get(&id.id) == Some(&head) {
        return;
    }
    // Only a record whose every link and seal holds is worth keeping.
    if diverge_desktop_room::check_record(&id.id, &moves).is_err() {
        return;
    }
    let _ = std::fs::create_dir_all(&state.records_dir);
    if std::fs::write(state.records_dir.join(format!("{}.json", id.id)), record.to_string()).is_ok() {
        state.record_heads.lock().unwrap().insert(id.id.clone(), head);
    }
}

/// Your copy of a room's record, if you hold one.
fn copy_of(state: &AppState, id: &str) -> Option<serde_json::Value> {
    std::fs::read_to_string(state.records_dir.join(format!("{id}.json"))).ok().and_then(|s| serde_json::from_str(&s).ok())
}

/// A room's moves as it serves them, or, when it can't be reached, as your copy holds them.
async fn moves_of(state: &AppState, id: &spaces::Id) -> (Vec<MoveView>, bool) {
    if let Some(moves) = read_json::<Vec<MoveView>>(state, id, diverge_desktop_room::room::FEED).await {
        keep_copy(state, id).await;
        return (moves, false);
    }
    let Some(copy) = copy_of(state, &id.id) else { return (Vec::new(), false) };
    let moves: Vec<diverge_desktop_room::Move> = serde_json::from_value(copy["moves"].clone()).unwrap_or_default();
    let views = moves
        .into_iter()
        .filter(|m| !(m.kind == "admitted" && m.args.get("listed").and_then(serde_json::Value::as_bool) == Some(false)))
        .map(|m| MoveView {
            id: m.id,
            kind: m.kind.clone(),
            author: m.author,
            by: m.by,
            agent_of: m.agent_of,
            at: m.at.to_rfc3339(),
            title: m.title,
            body: m.body,
            state: String::new(),
            parent: m.parent,
            fields: serde_json::Value::Object(m.fields),
            charter: m.charter,
            hash: m.hash,
        })
        .collect();
    (views, true)
}

#[tauri::command]
pub async fn spaces_home(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.spaces.home().await.map(|id| id.id))
}

#[tauri::command]
pub async fn home_feed(state: State<'_, AppState>) -> Result<Vec<HomeMove>, String> {
    let mut out = Vec::new();
    for e in state.spaces.list().await {
        let summary = summary(&e, &state.identity);
        let (moves, _) = moves_of(&state, &e.id).await;
        out.extend(moves.into_iter().map(|entry| HomeMove { space: summary.clone(), entry }));
    }
    out.sort_by(|a, b| b.entry.at.cmp(&a.entry.at));
    out.truncate(300);
    Ok(out)
}

fn personas(state: &AppState, entries: &[spaces::SpaceEntry]) -> Vec<PersonaView> {
    state
        .identity
        .personas()
        .into_iter()
        .map(|p| {
            let rooms = entries.iter().filter(|e| state.identity.in_room(&e.id.id).map(|q| q.id == p.id).unwrap_or(p.usual)).map(|e| e.title.clone()).collect();
            PersonaView { id: p.id, name: p.name, usual: p.usual, rooms }
        })
        .collect()
}

#[tauri::command]
pub async fn profile_get(state: State<'_, AppState>) -> Result<ProfileView, String> {
    let agents = listed(state.daemon.agents_list(agents::list::client::request::Frame {}).collect::<Vec<_>>().await).agents;
    let mine: Vec<String> = state.identity.personas().into_iter().map(|p| p.key).collect();
    let home = state.spaces.home().await.map(|id| id.id);
    let profile = state.spaces.profile().await.map(|id| id.id);
    let entries = state.spaces.list().await;
    let mut receipts = Vec::new();
    let mut shows = Vec::new();
    for e in &entries {
        let summary = summary(e, &state.identity);
        let (moves, _) = moves_of(&state, &e.id).await;
        for m in moves {
            if m.kind == "receipt" {
                let Some(statement) = m.fields.get("statement").and_then(|v| serde_json::from_value::<diverge_desktop_room::Statement>(v.clone()).ok()) else { continue };
                if !mine.iter().any(|k| Some(k.as_str()) == statement.field("to_person")) {
                    continue;
                }
                receipts.push(ReceiptView {
                    title: m.title.clone(),
                    for_title: m.body.strip_prefix("Completed: ").unwrap_or(&m.body).to_owned(),
                    space: summary.clone(),
                    to: statement.field("to_name").unwrap_or_default().to_owned(),
                    at: m.at.clone(),
                    issued_by: statement.field("host").unwrap_or_default().to_owned(),
                    holds: statement.holds(),
                    statement: serde_json::to_value(&statement).unwrap_or_default(),
                });
            } else if m.kind == "show" && Some(&e.id.id) == profile.as_ref() {
                shows.push(m);
            }
        }
    }
    receipts.sort_by(|a, b| b.at.cmp(&a.at));
    shows.sort_by(|a, b| b.at.cmp(&a.at));
    let machines = machines_with_volumes(&state).await;
    let volumes: Vec<VolumeView> = machines.iter().flat_map(|m| m.volumes.clone()).collect();
    let personas = personas(&state, &entries);
    Ok(ProfileView { receipts, shows, agents, machines, volumes, home, profile, personas })
}

#[tauri::command]
pub async fn people_list(state: State<'_, AppState>) -> Result<Vec<PersonView>, String> {
    // People are keys: the same key in two rooms is the same someone.
    let mut people: IndexMap<String, PersonView> = IndexMap::new();
    for e in state.spaces.list().await {
        let members: Vec<MemberView> = read_json(&state, &e.id, diverge_desktop_room::room::MEMBERS).await.unwrap_or_default();
        for m in members {
            if m.key.is_empty() || state.identity.owner_of(&m.key).is_some() {
                continue;
            }
            let p = people.entry(m.key.clone()).or_insert(PersonView { name: m.name.clone(), key: m.key.clone(), is_agent: m.is_agent, agent_of: m.agent_of.clone(), spaces: Vec::new() });
            p.spaces.push(e.id.id.clone());
        }
    }
    Ok(people.into_values().collect())
}

#[tauri::command]
pub async fn spaces_list(state: State<'_, AppState>) -> Result<Vec<SpaceSummary>, String> {
    Ok(state.spaces.list().await.iter().map(|e| summary(e, &state.identity)).collect())
}

#[tauri::command]
pub async fn spaces_get(state: State<'_, AppState>, id: String) -> Result<SpaceView, String> {
    let entries = state.spaces.list().await;
    let entry = entries.iter().find(|e| e.id.id == id).ok_or("no such Space")?;
    let sid = space_id(&id);
    let tools = state.spaces.tools(&sid).await.map_err(|e| e.message.to_string())?;
    let members: Vec<MemberView> = read_json(&state, &sid, diverge_desktop_room::room::MEMBERS).await.unwrap_or_default();
    let charter = read_text(&state, &sid, diverge_desktop_room::room::CHARTER).await.unwrap_or_default();
    Ok(SpaceView { summary: summary(entry, &state.identity), charter, members, tools: tools.tools.iter().map(Into::into).collect() })
}

#[tauri::command]
pub async fn spaces_feed(state: State<'_, AppState>, id: String) -> Result<FeedRead, String> {
    let sid = space_id(&id);
    Ok(match state.spaces.read(&sid, diverge_desktop_room::room::FEED).await {
        Ok(r) => {
            let moves = r.contents.iter().filter_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => serde_json::from_str::<Vec<MoveView>>(text).ok(), _ => None }).next().unwrap_or_default();
            keep_copy(&state, &sid).await;
            FeedRead::Feed { moves, from_copy: false }
        }
        Err(e) => match moves_of(&state, &sid).await {
            (moves, true) => FeedRead::Feed { moves, from_copy: true },
            _ => FeedRead::Error { message: e.message.to_string() },
        },
    })
}

/// Rooms this room vouches for: invites its host minted for the list.
#[tauri::command]
pub async fn spaces_doorways(state: State<'_, AppState>, id: String) -> Result<Vec<DoorwayView>, String> {
    Ok(read_json(&state, &space_id(&id), diverge_desktop_room::room::DOORWAYS).await.unwrap_or_default())
}

/// A vouch: your seal on someone's key, as text to hand them. They present
/// it when they knock; the host sees who vouched and whether it holds.
#[tauri::command]
pub fn vouch_for(state: State<'_, AppState>, key: String, name: String) -> Result<String, String> {
    use base64::Engine;
    let you = state.identity.usual();
    let statement = state.identity.state(&you.key, "vouch", serde_json::json!({ "for": key, "for_name": name, "by_name": you.name }))?;
    Ok(format!("diverge-vouch:{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&statement).unwrap_or_default())))
}

fn parse_vouch(text: &str) -> Result<diverge_desktop_room::Statement, String> {
    use base64::Engine;
    let body = text.trim().strip_prefix("diverge-vouch:").ok_or("that isn't a vouch")?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(body).map_err(|_| "that vouch is damaged".to_string())?;
    serde_json::from_slice(&bytes).map_err(|_| "that vouch is damaged".to_string())
}

/// Everyone a room you host let in and hasn't removed, listed or not: from its record.
#[tauri::command]
pub async fn spaces_admitted(state: State<'_, AppState>, id: String) -> Result<Vec<AdmittedView>, String> {
    let record: serde_json::Value = read_json(&state, &space_id(&id), diverge_desktop_room::room::RECORD).await.ok_or("the room can't be reached")?;
    let moves: Vec<diverge_desktop_room::Move> = serde_json::from_value(record["moves"].clone()).unwrap_or_default();
    let mut people: IndexMap<String, AdmittedView> = IndexMap::new();
    for m in moves {
        let key = m.args.get("key").and_then(serde_json::Value::as_str).unwrap_or_default().to_owned();
        match m.kind.as_str() {
            "admitted" => {
                let listed = m.args.get("listed").and_then(serde_json::Value::as_bool).unwrap_or(true);
                let is_agent = m.args.get("is_agent").and_then(serde_json::Value::as_bool).unwrap_or(false);
                people.insert(key.clone(), AdmittedView { name: m.title, key, listed, is_agent });
            }
            "removed" => {
                people.shift_remove(&key);
            }
            _ => {}
        }
    }
    Ok(people.into_values().collect())
}

/// Stop a room you host and run it again from its record: the way to close
/// its files to someone you removed.
#[tauri::command]
pub async fn spaces_restart(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.spaces.restart(&space_id(&id)).await.map_err(|e| error_text(&e))
}

/// Continue a room whose host is gone, on your machine, from your copy of
/// its record. The old moves keep their seals; you host what comes next.
#[tauri::command]
pub async fn spaces_continue(state: State<'_, AppState>, id: String) -> Result<HostOutcome, String> {
    use diverge_sdk::shared::containers::request::{Container, Image};
    let copy = copy_of(&state, &id).ok_or("you hold no copy of that room")?;
    let old: diverge_desktop_room::Args = serde_json::from_value(copy["args"].clone()).map_err(|e| e.to_string())?;
    let history: Vec<diverge_desktop_room::Move> = serde_json::from_value(copy["moves"].clone()).map_err(|e| e.to_string())?;
    let you = state.identity.in_room(&id).unwrap_or_else(|| state.identity.usual());
    let last = history.last().map(|m| m.hash.clone()).unwrap_or_default();
    let args = diverge_desktop_room::Args {
        id: String::new(),
        title: format!("{}, continued", old.title),
        kind: old.kind,
        host_key: you.key.clone(),
        host_name: you.name.clone(),
        charter: old.charter.clone(),
        open_door: old.open_door,
        continues: Some(diverge_desktop_room::Continues { room: old.id.clone(), title: old.title.clone(), last }),
    };
    let mut arguments = serde_json::to_value(&args).unwrap_or_default();
    arguments["history"] = serde_json::to_value(&history).unwrap_or_default();
    let container = Container { image: Image { name: "diverge-desktop-room".into(), digest: catalog::UNBUILT_DIGEST.into() }, memory: 1 << 30, disk: 1 << 30, volume_mounts: Vec::new(), fuse_file_mounts: Vec::new(), fuse_directory_mounts: Vec::new(), arguments };
    Ok(match state.spaces.host(container).await {
        Ok(new) => {
            state.identity.set_room(&new.id, &you.id);
            HostOutcome::Hosted { id: new.id }
        }
        Err(e) => HostOutcome::Error { message: error_text(&e) },
    })
}

#[tauri::command]
pub async fn spaces_call(state: State<'_, AppState>, id: String, tool: String, arguments: serde_json::Value) -> Result<CallOutcome, String> {
    Ok(match call_as_you(&state, &space_id(&id), &tool, arguments).await {
        Ok(result) => {
            let text = text_of(&result);
            if result.is_error == Some(true) { CallOutcome::Error { message: text } } else { CallOutcome::Ok { text } }
        }
        Err(message) => CallOutcome::Error { message },
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
    let Some(kind) = diverge_desktop_room::Kind::parse(&input.kind) else { return Ok(HostOutcome::Error { message: "no such kind of Space".into() }) };
    let you = state.identity.usual();
    let args = diverge_desktop_room::Args { id: String::new(), title: input.title, kind, host_key: you.key, host_name: you.name, charter: input.charter, open_door: input.open_door, continues: None };
    let container = Container {
        image: Image { name: "diverge-desktop-room".into(), digest: catalog::UNBUILT_DIGEST.into() },
        memory: 1 << 30,
        disk: 1 << 30,
        volume_mounts: Vec::new(),
        fuse_file_mounts: Vec::new(),
        fuse_directory_mounts: Vec::new(),
        arguments: serde_json::to_value(&args).unwrap_or_default(),
    };
    Ok(match state.spaces.host(container).await {
        Ok(id) => {
            state.identity.set_room(&id.id, "usual");
            HostOutcome::Hosted { id: id.id }
        }
        Err(e) => HostOutcome::Error { message: error_text(&e) },
    })
}

/// What an invite shows before you knock: nothing is sent.
#[tauri::command]
pub async fn spaces_door(state: State<'_, AppState>, invite: String) -> Result<DoorView, String> {
    let invite = spaces::Invite::from_text(&invite)?;
    let already_in = state.spaces.list().await.iter().any(|e| e.id.id == invite.id);
    Ok(DoorView {
        title: invite.title,
        kind: invite.kind,
        host_name: invite.host_name,
        host: (&invite.host).into(),
        charter: invite.charter,
        verbs: invite.verbs.into_iter().map(|v| VerbView { name: v.name, does: v.does }).collect(),
        invited: invite.secret.is_some(),
        already_in,
    })
}

#[tauri::command]
pub async fn spaces_join(state: State<'_, AppState>, invite: String, appear_as: AppearAs, note: String, listed: bool, vouch: Option<String>) -> Result<JoinOutcome, String> {
    let invite = spaces::Invite::from_text(&invite)?;
    let vouch = match vouch.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        Some(text) => Some(parse_vouch(text)?),
        None => None,
    };
    let persona = match appear_as {
        AppearAs::Usual => state.identity.usual(),
        AppearAs::Fresh { name } => state.identity.fresh(&name)?,
    };
    let knocking = spaces::Knocking { secret: invite.secret.clone(), key: persona.key.clone(), name: persona.name.clone(), note, listed, vouch };
    Ok(match state.spaces.join(&invite, &knocking).await {
        spaces::Joined::Joined(id) => {
            state.identity.set_room(&id.id, &persona.id);
            let about: Option<serde_json::Value> = read_json(&state, &id, diverge_desktop_room::room::ABOUT).await;
            let rules_match = about.and_then(|a| a["charter"].as_str().map(str::to_owned)).as_deref() == Some(diverge_desktop_room::seal::fingerprint(&invite.charter).as_str());
            JoinOutcome::Joined { id: id.id, rules_match }
        }
        spaces::Joined::Denied => JoinOutcome::Denied,
        spaces::Joined::Missing => JoinOutcome::Missing,
        spaces::Joined::Error(e) => JoinOutcome::Error { message: error_text(&e) },
    })
}

#[tauri::command]
pub async fn spaces_leave(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.spaces.leave(&space_id(&id)).await?;
    state.identity.forget_room(&id);
    let snapshot = state.tabs.lock().unwrap().close(&crate::tabs::key_of(&TabKind::Space { id }));
    let _ = app.emit("tabs://changed", snapshot);
    Ok(())
}

#[tauri::command]
pub async fn spaces_invite(state: State<'_, AppState>, id: String) -> Result<Option<InviteView>, String> {
    Ok(state.spaces.invite(&space_id(&id)).await.map(|i| InviteView { text: i.to_text() }))
}

/// A knock as the host reads it: the address the provider saw, and what the
/// knocker wrote, checked where it can be.
async fn knock_view(state: &AppState, k: &spaces::Knock) -> KnockView {
    let entries = state.spaces.list().await;
    let title = entries.iter().find(|e| e.id == k.space).map(|e| e.title.clone()).unwrap_or_else(|| k.space.id.clone());
    let knocking = spaces::Knocking::from_authorization(&k.authorize.authorization);
    let members: Vec<MemberView> = read_json(state, &k.space, diverge_desktop_room::room::MEMBERS).await.unwrap_or_default();
    let vouch = knocking.as_ref().and_then(|w| w.vouch.as_ref().map(|v| {
        let holds = v.kind == "vouch" && v.holds() && v.field("for") == Some(w.key.as_str());
        let by = members.iter().find(|m| m.key == v.key).map(|m| m.name.clone()).or_else(|| v.field("by_name").map(str::to_owned)).unwrap_or_else(|| "someone".into());
        VouchView { by, member_here: members.iter().any(|m| m.key == v.key), holds }
    }));
    KnockView {
        knock_id: k.knock_id,
        space: k.space.id.clone(),
        space_title: title,
        address: k.authorize.address.to_string(),
        name: knocking.as_ref().map(|w| w.name.clone()).unwrap_or_else(|| k.authorize.address.to_string()),
        note: knocking.as_ref().map(|w| w.note.clone()).unwrap_or_default(),
        listed: knocking.as_ref().map(|w| w.listed).unwrap_or(true),
        invited: knocking.as_ref().and_then(|w| w.secret.as_ref()).is_some(),
        vouch,
        at: k.at.to_rfc3339(),
    }
}

#[tauri::command]
pub fn knocks_watch(app: AppHandle, state: State<'_, AppState>, on_event: Channel<KnockEvent>) -> String {
    let (scope, token) = state.open_scope("knocks");
    let mut frames = state.spaces.knocks(token.clone());
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        while let Some(k) = frames.next().await {
            let view = knock_view(&state, &k).await;
            if on_event.send(KnockEvent::Knock { knock: view }).is_err() {
                token.cancel();
                return;
            }
        }
        let _ = on_event.send(KnockEvent::End);
    });
    scope
}

/// The host's answer; on yes, the host's `admit`, sealed as who they are there.
#[tauri::command]
pub async fn knocks_answer(state: State<'_, AppState>, knock_id: u64, yes: bool) -> Result<(), String> {
    let knock = state.spaces.answer(knock_id, if yes { spaces::Answer::Authorized } else { spaces::Answer::Denied }).await?;
    if yes {
        let knocking = spaces::Knocking::from_authorization(&knock.authorize.authorization).ok_or("that knock carried no key to let in")?;
        call_as_you(&state, &knock.space, "admit", serde_json::json!({ "key": knocking.key, "name": knocking.name, "listed": knocking.listed })).await?;
    }
    Ok(())
}

/// One ask, sent to several rooms at once: the same thread in each, so
/// Home follows it everywhere. There is no router; when there is, it's one
/// more place an ask can go.
#[tauri::command]
pub async fn asks_send(state: State<'_, AppState>, what: String, needs: Option<String>, ceiling: Option<String>, rooms: Vec<String>) -> Result<Vec<AskSent>, String> {
    if what.trim().is_empty() {
        return Err("an ask needs words".into());
    }
    let thread = diverge_desktop_room::seal::digest(format!("{what}{}", chrono::Utc::now()).as_bytes())[..12].to_owned();
    let mut out = Vec::new();
    for room in rooms {
        let args = serde_json::json!({ "what": what.trim(), "needs": needs, "ceiling": ceiling, "who_may_serve": "anyone", "thread": thread });
        let outcome = match call_as_you(&state, &space_id(&room), "ask", args).await {
            Ok(r) if r.is_error != Some(true) => CallOutcome::Ok { text: text_of(&r) },
            Ok(r) => CallOutcome::Error { message: text_of(&r) },
            Err(message) => CallOutcome::Error { message },
        };
        out.push(AskSent { room, outcome });
    }
    Ok(out)
}

// --- the table: a room's shared files ----------------------------------

fn parts(path: &str) -> Vec<String> {
    path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect()
}

#[tauri::command]
pub async fn table_tree(state: State<'_, AppState>, id: String) -> Result<VolumeTree, String> {
    Ok(match state.spaces.table_tree(&space_id(&id)).await {
        Ok(nodes) => VolumeTree::Tree { nodes: nodes.iter().map(Into::into).collect() },
        Err(e) => VolumeTree::Error { message: error_text(&e) },
    })
}

#[tauri::command]
pub async fn table_read(state: State<'_, AppState>, id: String, path: String) -> Result<FileRead, String> {
    Ok(match state.spaces.table_read(&space_id(&id), &parts(&path)).await {
        Ok(bytes) => {
            let size = bytes.len() as u64;
            match String::from_utf8(bytes) {
                Ok(text) => FileRead::Text { text, bytes: size },
                Err(_) => FileRead::Binary { bytes: size },
            }
        }
        Err(e) => FileRead::Error { message: error_text(&e) },
    })
}

#[tauri::command]
pub async fn table_write(state: State<'_, AppState>, id: String, path: String, text: String) -> Result<FileWritten, String> {
    Ok(match state.spaces.table_write(&space_id(&id), &parts(&path), text.into_bytes()).await {
        Ok(()) => FileWritten::Written,
        Err(e) => FileWritten::Error { message: error_text(&e) },
    })
}

/// A file from one room's table to another's, provider-side. Only between
/// rooms on one machine; otherwise it would pass through your Mac.
#[tauri::command]
pub async fn table_transfer(state: State<'_, AppState>, from: String, path: String, to: String) -> Result<FileWritten, String> {
    Ok(match state.spaces.transfer(&space_id(&from), &parts(&path), &space_id(&to)).await {
        Ok(()) => FileWritten::Written,
        Err(e) => FileWritten::Error { message: error_text(&e) },
    })
}

// --- personas and allowances -----------------------------------------------

#[tauri::command]
pub async fn personas_list(state: State<'_, AppState>) -> Result<Vec<PersonaView>, String> {
    let entries = state.spaces.list().await;
    Ok(personas(&state, &entries))
}

#[tauri::command]
pub fn persona_rename(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    state.identity.rename(&id, &name)
}

#[tauri::command]
pub fn allowance_get(state: State<'_, AppState>, id: String, agent: String) -> AllowanceView {
    let a = state.door.allowance(&id, &agent);
    AllowanceView { per_day: a.per_day, used_today: a.used }
}

#[tauri::command]
pub fn allowance_set(state: State<'_, AppState>, id: String, agent: String, per_day: u32) -> AllowanceView {
    state.door.set_allowance(&id, &agent, per_day);
    let a = state.door.allowance(&id, &agent);
    AllowanceView { per_day: a.per_day, used_today: a.used }
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
