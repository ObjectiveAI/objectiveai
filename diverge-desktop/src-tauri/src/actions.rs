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

use diverge_daemon_sdk::endpoints::{agents, filesystem};

use crate::catalog;
use crate::daemon::{Daemon, NewProvider, ProviderEntry, ReadFrame};
use crate::tabs::Tabs;
use crate::view::*;

pub struct AppState {
    pub daemon: Arc<dyn Daemon>,
    pub stand_in_host: Option<PathBuf>,
    pub scopes: Mutex<HashMap<String, CancellationToken>>,
    pub next_scope: AtomicU64,
    pub tabs: Mutex<Tabs>,
    pub views: Mutex<Vec<SavedView>>,
    pub views_file: PathBuf,
}

impl AppState {
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
    ("agents_message", "Send an agent a message; resolves when delivered"),
    ("agents_message_take_back", "Take back a message that has not been delivered yet"),
    ("logs_open", "Read an agent's log — filtered, reshaped by jq, perhaps watched"),
    ("scope_close", "Stop watching something"),
    ("files_tree_open", "Watch a folder on the daemon's machine"),
    ("files_read", "Open a file on the daemon's machine"),
    ("files_write", "Save a file on the daemon's machine"),
    ("machines_list", "List the machines the daemon can run on"),
    ("machines_add", "Add a machine: one you dial, or one that dials you"),
    ("machines_remove", "Remove a machine"),
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
    Ok(state.daemon.agents_create(request).await.into())
}

#[tauri::command]
pub async fn agents_delete(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<DeleteOutcome, String> {
    let outcome: DeleteOutcome = state.daemon.agents_delete(agents::delete::client::request::Frame { name: name.clone() }).await.into();
    if matches!(outcome, DeleteOutcome::Deleted) {
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

// --- files ---------------------------------------------------------------

#[tauri::command]
pub fn files_tree_open(state: State<'_, AppState>, path: String, on_event: Channel<TreeEvent>) -> String {
    let (id, token) = state.open_scope("tree");
    let mut frames = state.daemon.filesystem_filetree(filesystem::filetree::client::request::Frame { path }, token.clone());
    tauri::async_runtime::spawn(async move {
        while let Some(frame) = frames.next().await {
            if on_event.send(TreeEvent::from(frame)).is_err() {
                token.cancel();
                return;
            }
        }
        let _ = on_event.send(TreeEvent::End);
    });
    id
}

#[tauri::command]
pub async fn files_read(state: State<'_, AppState>, path: String) -> Result<FileRead, String> {
    let mut frames = state.daemon.filesystem_read(filesystem::read::client::request::Frame { path });
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
pub async fn files_write(state: State<'_, AppState>, path: String, text: String) -> Result<FileWritten, String> {
    Ok(state.daemon.filesystem_write(filesystem::write::client::request::Frame { path }, text.into_bytes()).await.into())
}

// --- machines (ours until the wire has them) -----------------------------

fn machine(entry: ProviderEntry) -> MachineView {
    MachineView { identity: (&entry.identity).into(), volumes: entry.volumes, added: entry.added.to_rfc3339() }
}

#[tauri::command]
pub async fn machines_list(state: State<'_, AppState>) -> Result<Vec<MachineView>, String> {
    Ok(state.daemon.providers_list().await.into_iter().map(machine).collect())
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
