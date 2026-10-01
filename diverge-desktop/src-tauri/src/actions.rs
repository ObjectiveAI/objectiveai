//! The action registry: everything a person can do in this app, in one
//! place. The page calls these; an agent's door (the MCP server in
//! `door.rs`) calls the same functions. No action lives only in a click handler.

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
    /// Whether anything answers for the daemon, machines and rooms.
    pub network: bool,
    /// The folder every file the app keeps is in.
    pub data: PathBuf,
    /// Whether this copy of the app holds that folder. One that doesn't
    /// changes nothing in it: it saves nothing and sends nothing.
    pub folder: crate::store::Hold,
    pub scopes: Mutex<HashMap<String, CancellationToken>>,
    pub next_scope: AtomicU64,
    pub tabs: Mutex<Tabs>,
    pub views: Mutex<Vec<SavedView>>,
    /// Machine names, yours: identity key → name.
    pub machine_names: Mutex<HashMap<String, String>>,
    /// Your copies of every room's record, one file each, kept as you go.
    pub records_dir: PathBuf,
    /// The last hash of each copy, so a copy is written only when it grew.
    pub record_heads: Mutex<HashMap<String, String>>,
    /// Your asks' thread ids, one per room they went to, and the one thread
    /// each belongs to. Only this app knows they're one; the rooms can't tell.
    pub threads: Mutex<HashMap<String, String>>,
    /// What this app last stated each agent mounts: agent name → mounts.
    pub agent_mounts: Mutex<HashMap<String, AgentMounts>>,
    /// What waits for the first-run page to be finished: anything that acts as you.
    pub after_first_run: Mutex<Vec<AfterFirstRun>>,
}

/// Something that waits for the first-run page to be finished.
pub type AfterFirstRun = Box<dyn FnOnce(&AppState) + Send>;

/// The files the app keeps, in its folder.
const KEYS_FILE: &str = "identity.json";
const ALLOWANCES_FILE: &str = "allowances.json";
const VIEWS_FILE: &str = "views.json";
const MACHINE_NAMES_FILE: &str = "machine_names.json";
const AGENT_MOUNTS_FILE: &str = "agent_mounts.json";
const THREADS_FILE: &str = "threads.json";
const RECORDS_DIR: &str = "records";

/// The variable that puts the app's files in a folder of your choosing:
/// for tests, and for a second copy of the app beside the first.
pub const DATA_DIR_VAR: &str = "DIVERGE_DATA_DIR";

/// Where the app keeps its files: the folder [`DATA_DIR_VAR`] names, when
/// it names one (a relative one from where the app started); the system's
/// place for this app otherwise.
pub fn data_dir<E>(from_env: Option<std::ffi::OsString>, system: impl FnOnce() -> Result<PathBuf, E>) -> Result<PathBuf, E> {
    match from_env.filter(|v| !v.is_empty()).map(PathBuf::from) {
        Some(dir) => Ok(std::path::absolute(&dir).unwrap_or(dir)),
        None => system(),
    }
}

/// What answers the app's seams, and what the app knows of them before it
/// has files of its own.
pub struct Seams {
    pub daemon: Arc<dyn Daemon>,
    pub machines: Arc<dyn Machines>,
    pub spaces: Arc<dyn Spaces>,
    pub stand_in_host: Option<PathBuf>,
    pub network: bool,
    /// What the app counts as having stated each agent mounts, until it
    /// keeps a file of its own.
    pub first_mounts: HashMap<String, AgentMounts>,
}

impl AppState {
    /// The app over its folder: your keys, every file it keeps, and
    /// whatever answers the seams — the stand-in, in a build with it;
    /// nothing, otherwise, and the screens say so. Starts the reporter and
    /// the hires. Call it inside the runtime the app keeps.
    ///
    /// One copy of the app at a time uses a folder. A copy that finds it
    /// held by another (or can't tell) reads your keys and files as they
    /// are, changes nothing there, and answers nothing; the screens say why.
    pub async fn open(data: PathBuf) -> AppState {
        let folder = crate::store::hold(&data);
        let state = match folder.refusal() {
            Some(says) => {
                let identity = Arc::new(crate::identity::Identity::untouched(&data.join(KEYS_FILE), says));
                let absent = Arc::new(crate::absent::Absent::saying(says));
                let seams = Seams { daemon: absent.clone(), machines: absent.clone(), spaces: absent, stand_in_host: None, network: false, first_mounts: HashMap::new() };
                return Self::assemble(data, identity, seams, folder);
            }
            None => {
                // Nobody is named from this Mac: until the first-run page is finished there's no you here.
                let identity = Arc::new(crate::identity::Identity::open(data.join(KEYS_FILE)));
                #[cfg(feature = "stand-in")]
                let state = Self::with_stand_in(data, identity, folder);
                #[cfg(not(feature = "stand-in"))]
                let state = {
                    let absent = Arc::new(crate::absent::Absent::not_yet());
                    let seams = Seams { daemon: absent.clone(), machines: absent.clone(), spaces: absent, stand_in_host: None, network: false, first_mounts: HashMap::new() };
                    Self::assemble(data, identity, seams, folder)
                };
                state
            }
        };
        crate::reporter::spawn(state.daemon.clone(), state.spaces.clone(), state.identity.clone());
        crate::hires::spawn(state.daemon.clone(), state.spaces.clone(), state.identity.clone(), state.door.clone());
        state
    }

    /// The stand-in daemon and rooms, their past, and their scenes.
    #[cfg(feature = "stand-in")]
    fn with_stand_in(data: PathBuf, identity: Arc<crate::identity::Identity>, folder: crate::store::Hold) -> AppState {
        use crate::daemon::stub::StubDaemon;
        use crate::spaces::stub::StubSpaces;
        let host = data.join("stand-in-host");
        let daemon = Arc::new(StubDaemon::new(host.clone()));
        let stub = StubSpaces::new(identity.clone(), host.join("tables"));
        // On a first run, the stand-in's seeded agents count as made here.
        let first_mounts = daemon.creates().iter().map(|c| (c.name.clone(), AgentMounts::of_create(c))).collect();
        // The stand-in answers both seams: it knows every agent's mounts, so it keeps every machine's holds.
        let seams = Seams { daemon: daemon.clone(), machines: daemon.clone(), spaces: Arc::new(stub.clone()), stand_in_host: Some(host), network: true, first_mounts };
        let state = Self::assemble(data, identity, seams, folder);
        daemon.set_door(state.door.clone());
        // The stand-in's rooms are yours and others': they start once there's a you, after the first-run page.
        let begin = move |state: &AppState| {
            stub.begin();
            // The stand-in's past: copies you'd already hold of rooms that have since gone quiet.
            for (id, record) in stub.records_you_hold() {
                let file = record_file(state, &id);
                if !file.exists() {
                    let _ = crate::store::save(&file, crate::store::RECORD_COPY, &record);
                }
            }
            // The stand-in's own scenes: someone hires one of your agents a little after you open the app.
            stub.stage();
        };
        if state.identity.ready().is_ok() {
            begin(&state);
        } else {
            state.after_first_run.lock().unwrap().push(Box::new(begin));
        }
        state
    }

    /// The app over its folder and these seams, every file it keeps read
    /// back. Call it inside a tokio runtime: the door keeps that one. In a
    /// folder this copy doesn't hold, files are read as they are, touching
    /// nothing, and nothing is written.
    pub fn assemble(data: PathBuf, identity: Arc<crate::identity::Identity>, seams: Seams, folder: crate::store::Hold) -> AppState {
        use crate::store;
        let held = folder.held();
        fn kept<T: serde::de::DeserializeOwned>(held: bool, file: &std::path::Path, format: store::Format) -> Option<T> {
            if held { store::load(file, format) } else { store::peek(file, format) }
        }
        // An allowance lets an agent act without asking: a copy that can't save one starts with none.
        let door = Arc::new(Door::new(seams.spaces.clone(), identity.clone(), held.then(|| data.join(ALLOWANCES_FILE))));
        let records_dir = data.join(RECORDS_DIR);
        if held {
            let _ = std::fs::create_dir_all(&records_dir);
        }
        AppState {
            daemon: seams.daemon,
            identity,
            machines: seams.machines,
            spaces: seams.spaces,
            door,
            stand_in_host: seams.stand_in_host,
            network: seams.network,
            scopes: Mutex::new(HashMap::new()),
            next_scope: AtomicU64::new(1),
            tabs: Mutex::new(Tabs::default()),
            views: Mutex::new(kept(held, &data.join(VIEWS_FILE), store::VIEWS).unwrap_or_default()),
            machine_names: Mutex::new(kept(held, &data.join(MACHINE_NAMES_FILE), store::MACHINE_NAMES).unwrap_or_default()),
            records_dir,
            record_heads: Mutex::new(HashMap::new()),
            threads: Mutex::new(kept(held, &data.join(THREADS_FILE), store::THREADS).unwrap_or_default()),
            agent_mounts: Mutex::new(kept(held, &data.join(AGENT_MOUNTS_FILE), store::AGENT_MOUNTS).unwrap_or(seams.first_mounts)),
            after_first_run: Mutex::new(Vec::new()),
            data,
            folder,
        }
    }

    /// Write one of the app's files, whole or not at all; nothing, in a
    /// folder this copy doesn't hold.
    fn keep<T: serde::Serialize + serde::de::DeserializeOwned>(&self, name: &str, format: crate::store::Format, data: &T) {
        if self.folder.held() {
            let _ = crate::store::save(&self.data.join(name), format, data);
        }
    }

    fn remember_mounts(&self, name: &str, mounts: Option<AgentMounts>) {
        let mut all = self.agent_mounts.lock().unwrap();
        match mounts {
            Some(m) => all.insert(name.to_owned(), m),
            None => all.remove(name),
        };
        self.keep(AGENT_MOUNTS_FILE, crate::store::AGENT_MOUNTS, &*all);
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
        self.keep(VIEWS_FILE, crate::store::VIEWS, &views.to_vec());
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
    ("asks_close", "Close one of your asks in every room it went to"),
    ("spaces_doorways", "The rooms a room vouches for"),
    ("vouch_for", "Vouch for someone into a room you're in: your word, for that room, for a week, to hand them"),
    ("identity_broken", "Whether your keys file can be used; if not, where it is and whether a newer version wrote it"),
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
    ("app_info", "Whether the daemon is the stand-in, whether there's a network part at all, and the contract pin"),
    ("files_set_aside", "Files the app couldn't use since it started: each set aside untouched, or left where it is; what it carried on from"),
    ("first_run_get", "Where the first-run page stands: finished, not started, or a folder from before accounts"),
    ("first_run_finish", "Finish the first-run page: the name people should call you, and that you're 18 or older. Makes your account; nothing is signed before"),
];

#[tauri::command]
pub fn actions_list() -> Vec<ActionInfo> {
    REGISTRY.iter().map(|(name, does)| ActionInfo { name: (*name).into(), does: (*does).into() }).collect()
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    info(&state)
}

fn info(state: &AppState) -> AppInfo {
    AppInfo {
        stand_in: state.stand_in_host.is_some(),
        network: state.network,
        contract_pin: include_str!("../../CONTRACT_PIN").lines().next().unwrap_or_default().to_owned(),
        stand_in_host: state.stand_in_host.as_ref().map(|p| p.display().to_string()),
        folder: state.data.display().to_string(),
        folder_held: (&state.folder).into(),
    }
}

/// Files the app couldn't use since it started: each set aside, untouched,
/// under a new name, or left where it is; and what the app carried on from.
#[tauri::command]
pub async fn files_set_aside(state: State<'_, AppState>) -> Result<Vec<FileNoticeView>, String> {
    Ok(file_notices(&state).await)
}

async fn file_notices(state: &AppState) -> Vec<FileNoticeView> {
    let notices = crate::store::notices_under(&state.data);
    // A record copy is named for a digest of its room's id: name the room, where the app knows it.
    let rooms: HashMap<String, String> = if notices.iter().any(|n| n.kind == crate::store::RECORD_COPY.name) {
        state.spaces.list().await.into_iter().map(|e| (record_file_name(&e.id.id), e.title)).collect()
    } else {
        HashMap::new()
    };
    notices
        .iter()
        .map(|n| {
            let room = (n.kind == crate::store::RECORD_COPY.name).then(|| n.slot.file_name().and_then(|f| rooms.get(f.to_string_lossy().as_ref())).cloned()).flatten();
            FileNoticeView::of(n, room)
        })
        .collect()
}

/// Where the first-run page stands. Until it's finished there's no you
/// here: nothing is signed or sent, and the screens show only that page.
#[tauri::command]
pub fn first_run_get(state: State<'_, AppState>) -> FirstRunView {
    state.identity.first_run().into()
}

/// Finish the first-run page. Only a person does this, on the page: no
/// door gives an agent your name, your account or your words.
#[tauri::command]
pub async fn first_run_finish(state: State<'_, AppState>, name: String, adult: bool) -> Result<FirstRunView, String> {
    finish_first_run(&state, &name, adult)
}

fn finish_first_run(state: &AppState, name: &str, adult: bool) -> Result<FirstRunView, String> {
    state.identity.finish_first_run(name, adult)?;
    let waiting = std::mem::take(&mut *state.after_first_run.lock().unwrap());
    for then in waiting {
        then(state);
    }
    Ok(state.identity.first_run().into())
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
    create_agent(&state, input).await
}

async fn create_agent(state: &AppState, input: CreateAgentInput) -> Result<CreateOutcome, String> {
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
            let _ = admit_agent(state, &home, &name).await;
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

/// Your keys file, if it can't be used: where it is, and whether a newer
/// version of the app wrote it. The app then signs nothing.
#[tauri::command]
pub fn identity_broken(state: State<'_, AppState>) -> Option<KeysBrokenView> {
    state.identity.broken().map(|(file, newer)| KeysBrokenView { file: file.display().to_string(), newer })
}

/// Your call to a room: sealed as whoever you are there, one call at a time for that key and room.
async fn call_as_you(state: &AppState, id: &spaces::Id, tool: &str, arguments: serde_json::Value) -> Result<rmcp::model::CallToolResult, String> {
    let mut params = rmcp::model::CallToolRequestParams::new(tool.to_owned()).with_arguments(arguments.as_object().cloned().unwrap_or_default());
    let actor = state.identity.you_in(&id.id);
    let turn = state.identity.turn(&actor, &id.id);
    let _held = turn.lock().await;
    state.identity.seal(&actor, &id.id, &mut params)?;
    state.spaces.call(id, params).await.map_err(|e| e.message.to_string())
}

/// Let one of your agents into a room you host, tethered to who you are there.
pub async fn admit_agent(state: &AppState, room: &spaces::Id, agent: &str) -> Result<(), String> {
    let a = state.identity.agent_in(agent, Some(&room.id))?;
    let person = state.identity.who_in(&room.id)?;
    call_as_you(state, room, "admit", serde_json::json!({ "key": a.key, "name": a.name, "is_agent": true, "agent_of": person.key, "tether": a.tether })).await.map(|_| ())
}

/// Where your copy of a room's record lives: named for a digest of the
/// room's id, never the id itself, which came from someone else.
pub fn record_file_name(id: &str) -> String {
    format!("{}.json", &diverge_desktop_room::seal::digest(id.as_bytes())[..32])
}

fn record_file(state: &AppState, id: &str) -> std::path::PathBuf {
    state.records_dir.join(record_file_name(id))
}

/// Keep your copy of a room's record whenever it has grown, if it is that
/// room's and it replays whole.
async fn keep_copy(state: &AppState, id: &spaces::Id) {
    let Some(record) = read_json::<diverge_desktop_room::Record>(state, id, diverge_desktop_room::room::RECORD).await else { return };
    let head = record.moves.last().map(|m| m.hash.clone()).unwrap_or_default();
    if state.record_heads.lock().unwrap().get(&id.id) == Some(&head) {
        return;
    }
    if !state.folder.held() || record.args.id != id.id || diverge_desktop_room::Room::check(&record).is_err() {
        return;
    }
    if crate::store::save(&record_file(state, &id.id), crate::store::RECORD_COPY, &record).is_ok() {
        state.record_heads.lock().unwrap().insert(id.id.clone(), head);
    }
}

/// Your copy of a room's record, if you hold one that is that room's and
/// replays whole. One that doesn't is set aside, and the copy before it
/// carries on.
fn copy_of(state: &AppState, id: &str) -> Option<diverge_desktop_room::Record> {
    let check = |record: &diverge_desktop_room::Record| {
        if record.args.id != id {
            return Err("another room's record".into());
        }
        diverge_desktop_room::Room::check(record).map(|_| ())
    };
    if state.folder.held() {
        crate::store::load_with(&record_file(state, id), crate::store::RECORD_COPY, check)
    } else {
        crate::store::peek_with(&record_file(state, id), crate::store::RECORD_COPY, check)
    }
}

/// Your copy of a room, rebuilt by replay: what it served when you last saw it.
fn copy_room(state: &AppState, id: &str) -> Option<diverge_desktop_room::Room> {
    copy_of(state, id).and_then(|record| diverge_desktop_room::Room::check(&record).ok())
}

/// A room's moves as it serves them, or, when it can't be reached, as your copy holds them.
async fn moves_of(state: &AppState, id: &spaces::Id) -> (Vec<MoveView>, bool) {
    if let Some(moves) = read_json::<Vec<MoveView>>(state, id, diverge_desktop_room::room::FEED).await {
        keep_copy(state, id).await;
        return (moves, false);
    }
    let Some(room) = copy_room(state, &id.id) else { return (Vec::new(), false) };
    let moves = room
        .read(diverge_desktop_room::room::FEED)
        .ok()
        .and_then(|r| r.contents.into_iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => serde_json::from_str::<Vec<MoveView>>(&text).ok(), _ => None }))
        .unwrap_or_default();
    (moves, true)
}

/// A new room's container arguments: its settings signed as `host`, the key
/// made for it (the program's alone), and the record it continues, if any.
fn room_arguments(state: &AppState, mut args: diverge_desktop_room::Args, room_key: &diverge_desktop_room::Keypair, before: Option<diverge_desktop_room::Record>) -> Result<serde_json::Value, String> {
    args.sig = state.identity.state(&args.host_key, "room", args.body())?.sig;
    let mut arguments = serde_json::to_value(&args).map_err(|e| e.to_string())?;
    arguments["room_secret"] = serde_json::json!(room_key.secret_hex());
    if let Some(before) = before {
        arguments["before"] = serde_json::to_value(before).map_err(|e| e.to_string())?;
    }
    Ok(arguments)
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
    // Your own asks, each room's thread id mapped back to the one thread you sent.
    let threads = state.threads.lock().unwrap().clone();
    for m in out.iter_mut().filter(|m| m.entry.kind == "ask") {
        let base = m.entry.fields.get("thread").and_then(serde_json::Value::as_str).and_then(|t| threads.get(t)).cloned();
        if let (Some(base), Some(fields)) = (base, m.entry.fields.as_object_mut()) {
            fields.insert("thread".into(), serde_json::json!(base));
        }
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
            let rooms = entries.iter().filter(|e| state.identity.in_room(&e.id.id).map(|q| q.id == p.id).unwrap_or(p.usual)).map(|e| e.id.id.clone()).collect();
            PersonaView { id: p.id, name: p.name, usual: p.usual, rooms }
        })
        .collect()
}

#[tauri::command]
pub async fn profile_get(state: State<'_, AppState>) -> Result<ProfileView, String> {
    let agents = listed(state.daemon.agents_list(agents::list::client::request::Frame {}).collect::<Vec<_>>().await).agents;
    let mine = state.identity.personas();
    let home = state.spaces.home().await.map(|id| id.id);
    let profile = state.spaces.profile().await.map(|id| id.id);
    let entries = state.spaces.list().await;
    // Everyone you've met: members of rooms you're in.
    let mut met = std::collections::HashSet::new();
    for e in &entries {
        let members: Vec<MemberView> = read_json(&state, &e.id, diverge_desktop_room::room::MEMBERS).await.unwrap_or_default();
        met.extend(members.into_iter().map(|m| m.key));
    }
    let mut receipts = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut shows = Vec::new();
    for e in &entries {
        let (moves, _) = moves_of(&state, &e.id).await;
        for m in moves {
            if m.kind == "receipt" {
                let Some(statement) = m.fields.get("statement").and_then(|v| serde_json::from_value::<diverge_desktop_room::Statement>(v.clone()).ok()) else { continue };
                let Some(earned_as) = mine.iter().find(|p| Some(p.key.as_str()) == statement.field("to_person")) else { continue };
                // One receipt, once: a continued room carries its old receipts too.
                if !seen.insert(statement.sig.clone()) {
                    continue;
                }
                let room = statement.field("room").unwrap_or_default().to_owned();
                receipts.push(ReceiptView {
                    title: m.title.clone(),
                    for_title: m.body.strip_prefix("Completed: ").unwrap_or(&m.body).to_owned(),
                    room_title: statement.field("room_title").unwrap_or_default().to_owned(),
                    space: entries.iter().find(|x| x.id.id == room).map(|x| summary(x, &state.identity)),
                    to: statement.field("to_name").unwrap_or_default().to_owned(),
                    at: m.at.clone(),
                    issued_by: statement.field("host").unwrap_or_default().to_owned(),
                    known: met.contains(&statement.key),
                    holds: statement.holds() && diverge_desktop_room::id_holds(&room, &statement.key),
                    earned_as: earned_as.name.clone(),
                    earned_as_usual: earned_as.usual,
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
    space_view(&state, id).await
}

async fn space_view(state: &AppState, id: String) -> Result<SpaceView, String> {
    let entries = state.spaces.list().await;
    let entry = entries.iter().find(|e| e.id.id == id).ok_or("no such Space")?;
    let sid = space_id(&id);
    let tools = state.spaces.tools(&sid).await.map_err(|e| e.message.to_string())?;
    let mut members: Vec<MemberView> = read_json(state, &sid, diverge_desktop_room::room::MEMBERS).await.unwrap_or_default();
    let mut charter = read_text(state, &sid, diverge_desktop_room::room::CHARTER).await.unwrap_or_default();
    // Unreachable: who was here and the rules, as your copy has them.
    if !entry.online {
        if let Some(room) = copy_room(state, &id) {
            let text = |uri: &str| room.read(uri).ok().and_then(|r| r.contents.into_iter().find_map(|c| match c { rmcp::model::ResourceContents::TextResourceContents { text, .. } => Some(text), _ => None }));
            members = text(diverge_desktop_room::room::MEMBERS).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            charter = room.charter().to_owned();
        }
    }
    crate::marks::members(&id, &mut members);
    let before = if entry.mine { people_from_before(state, &sid, &entries, &members).await } else { Vec::new() };
    Ok(SpaceView { summary: summary(entry, &state.identity), charter, members, tools: tools.tools.iter().map(Into::into).collect(), before })
}

/// For a room you continued: who was listed in the room before and isn't
/// here, each with a direct room you share, if any.
async fn people_from_before(state: &AppState, id: &spaces::Id, entries: &[spaces::SpaceEntry], here: &[MemberView]) -> Vec<BeforeView> {
    let Some(record) = read_json::<diverge_desktop_room::Record>(state, id, diverge_desktop_room::room::RECORD).await else { return Vec::new() };
    let Some(before) = record.before else { return Vec::new() };
    let Ok(old) = diverge_desktop_room::Room::check(&before) else { return Vec::new() };
    let mut out = Vec::new();
    for m in old.members().filter(|m| m.listed && !m.removed && !m.is_agent) {
        if state.identity.owner_of(&m.key).is_some() || here.iter().any(|h| h.key == m.key) {
            continue;
        }
        let mut dm = None;
        for e in entries.iter().filter(|e| e.kind == "dm") {
            let members: Vec<MemberView> = read_json(state, &e.id, diverge_desktop_room::room::MEMBERS).await.unwrap_or_default();
            if members.iter().any(|x| x.key == m.key) {
                dm = Some(e.id.id.clone());
                break;
            }
        }
        out.push(BeforeView { name: m.name.clone(), key: m.key.clone(), dm });
    }
    out
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

/// A vouch: your word for someone's key, into one room you're in, for a
/// week, as text to hand them. They present it when they knock there; the
/// host sees who vouched, that they're a member, and that it holds. You
/// vouch as the name you go by in that room.
#[tauri::command]
pub async fn vouch_for(state: State<'_, AppState>, key: String, name: String, room: String) -> Result<String, String> {
    use base64::Engine;
    if !state.spaces.list().await.iter().any(|e| e.id.id == room) {
        return Err("you can vouch someone into a room you're in".into());
    }
    let you = state.identity.who_in(&room)?;
    let until = (chrono::Utc::now() + spaces::VOUCH_GOOD_FOR).to_rfc3339();
    let statement = state.identity.state(&you.key, "vouch", serde_json::json!({ "for": key, "for_name": name, "by_name": you.name, "room": room, "until": until }))?;
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
    admitted(&state, &id).await
}

async fn admitted(state: &AppState, id: &str) -> Result<Vec<AdmittedView>, String> {
    let record: diverge_desktop_room::Record = read_json(state, &space_id(id), diverge_desktop_room::room::RECORD).await.ok_or("the room can't be reached")?;
    let mut people: IndexMap<String, AdmittedView> = IndexMap::new();
    // Someone let in unlisted is known by a mark until they act; then by their key.
    let acted: HashMap<String, String> = record.moves.iter().map(|m| (diverge_desktop_room::key_mark(&record.args.id, &m.by), m.by.clone())).collect();
    for m in record.moves {
        let key = m.args.get("key").or_else(|| m.args.get("key_mark")).and_then(serde_json::Value::as_str).unwrap_or_default().to_owned();
        let key = acted.get(&key).cloned().unwrap_or(key);
        match m.kind.as_str() {
            "admitted" => {
                let listed = m.args.get("listed").and_then(serde_json::Value::as_bool).unwrap_or(true);
                let is_agent = m.args.get("is_agent").and_then(serde_json::Value::as_bool).unwrap_or(false);
                let yours = state.identity.owner_of(&key).is_some();
                people.insert(key.clone(), AdmittedView { name: m.title, key, listed, is_agent, yours, mark: None });
            }
            "removed" => {
                // A person's agents leave in the same move.
                let also: Vec<String> = m.fields.get("also").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
                for k in also.iter().chain(std::iter::once(&key)) {
                    people.shift_remove(k);
                }
            }
            _ => {}
        }
    }
    let mut people: Vec<AdmittedView> = people.into_values().collect();
    let who: Vec<(&str, &str)> = people.iter().map(|p| (p.name.as_str(), p.key.as_str())).collect();
    let marks = crate::marks::shared_names(id, &who);
    for (p, mark) in people.iter_mut().zip(marks) {
        p.mark = mark;
    }
    Ok(people)
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
    let old = diverge_desktop_room::Room::check(&copy)?;
    let you = state.identity.who_in(&id)?;
    let room_key = diverge_desktop_room::Keypair::generate();
    let args = diverge_desktop_room::Args {
        id: diverge_desktop_room::room_id(&diverge_desktop_room::fresh_label(), &you.key),
        title: format!("{}, continued", old.args.title),
        kind: old.args.kind,
        host_key: you.key.clone(),
        host_name: you.name.clone(),
        charter: old.charter().to_owned(),
        open_door: old.args.open_door,
        continues: Some(diverge_desktop_room::Continues { room: old.args.id.clone(), title: old.args.title.clone(), last: old.last_hash() }),
        room_key: room_key.key(),
        at: chrono::Utc::now(),
        rules: 1,
        host_account: None,
        sig: String::new(),
    };
    let arguments = room_arguments(&state, args, &room_key, Some(copy))?;
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
    host_space(&state, input).await
}

async fn host_space(state: &AppState, input: HostSpaceInput) -> Result<HostOutcome, String> {
    use diverge_sdk::shared::containers::request::{Container, Image};
    let Some(kind) = diverge_desktop_room::Kind::parse(&input.kind) else { return Ok(HostOutcome::Error { message: "no such kind of Space".into() }) };
    let you = state.identity.usual()?;
    let room_key = diverge_desktop_room::Keypair::generate();
    let args = diverge_desktop_room::Args {
        id: diverge_desktop_room::room_id(&diverge_desktop_room::fresh_label(), &you.key),
        title: input.title,
        kind,
        host_key: you.key,
        host_name: you.name,
        charter: input.charter,
        open_door: input.open_door,
        continues: None,
        room_key: room_key.key(),
        at: chrono::Utc::now(),
        rules: 1,
        host_account: None,
        sig: String::new(),
    };
    let arguments = room_arguments(state, args, &room_key, None)?;
    let container = Container {
        image: Image { name: "diverge-desktop-room".into(), digest: catalog::UNBUILT_DIGEST.into() },
        memory: 1 << 30,
        disk: 1 << 30,
        volume_mounts: Vec::new(),
        fuse_file_mounts: Vec::new(),
        fuse_directory_mounts: Vec::new(),
        arguments,
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
    door_of(&state, &invite).await
}

async fn door_of(state: &AppState, invite: &str) -> Result<DoorView, String> {
    let invite = spaces::Invite::from_text(invite)?;
    let already_in = state.spaces.list().await.iter().any(|e| e.id.id == invite.id);
    Ok(DoorView {
        usual_name: state.identity.usual()?.name,
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
        AppearAs::Usual => state.identity.usual()?,
        AppearAs::Fresh { name } => state.identity.fresh(&name)?,
    };
    let now = chrono::Utc::now();
    if let Some(v) = &vouch {
        if !spaces::vouch_holds(v, &persona.key, &invite.id, now) {
            return Err("that vouch isn't for you at this room, or it has run out".into());
        }
    }
    let knocking = spaces::Knocking {
        room: invite.id.clone(),
        invite: invite.secret.as_deref().map(|s| spaces::Knocking::invite_mark(&invite.id, s)),
        key: persona.key.clone(),
        name: persona.name.clone(),
        note,
        listed,
        vouch,
        at: now,
        sig: String::new(),
    };
    let knocking = spaces::Knocking { sig: state.identity.state(&persona.key, "knock", knocking.body())?.sig, ..knocking };
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
    let secret = state.spaces.invite(&k.space).await.and_then(|i| i.secret);
    let members: Vec<MemberView> = read_json(state, &k.space, diverge_desktop_room::room::MEMBERS).await.unwrap_or_default();
    knock_view_of(k, title, secret.as_deref(), &members, chrono::Utc::now())
}

/// A knock, checked against the room's current invite and its members.
pub fn knock_view_of(k: &spaces::Knock, title: String, secret: Option<&str>, members: &[MemberView], now: chrono::DateTime<chrono::Utc>) -> KnockView {
    let knocking = spaces::Knocking::from_authorization(&k.authorize.authorization);
    // Nothing a knock says counts until it checks: signed by its key, for this room, lately.
    let checked = knocking.as_ref().is_some_and(|w| w.check(&k.space.id, now).is_ok());
    let invited = checked && knocking.as_ref().and_then(|w| w.invite.as_deref()).is_some_and(|mark| secret.is_some_and(|s| mark == spaces::Knocking::invite_mark(&k.space.id, s)));
    let vouch = knocking.as_ref().and_then(|w| w.vouch.as_ref().map(|v| {
        let holds = checked && spaces::vouch_holds(v, &w.key, &k.space.id, now);
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
        invited,
        checked,
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

/// The host's answer. On yes: the knock is checked again, the host's `admit`
/// (sealed as who they are there) goes first, and the door opens only once
/// the room has let them in. If anything fails, the answer is no.
#[tauri::command]
pub async fn knocks_answer(state: State<'_, AppState>, knock_id: u64, yes: bool) -> Result<(), String> {
    answer_knock(&state, knock_id, yes).await
}

async fn answer_knock(state: &AppState, knock_id: u64, yes: bool) -> Result<(), String> {
    if !yes {
        state.spaces.answer(knock_id, spaces::Answer::Denied).await?;
        return Ok(());
    }
    let knock = state.spaces.pending(knock_id).await.ok_or("nobody is at that door any more")?;
    let knocking = spaces::Knocking::from_authorization(&knock.authorize.authorization).filter(|w| w.check(&knock.space.id, chrono::Utc::now()).is_ok());
    let Some(knocking) = knocking else {
        state.spaces.answer(knock_id, spaces::Answer::Denied).await?;
        return Err("that knock doesn't check, so it wasn't let in".into());
    };
    let args = if knocking.listed {
        serde_json::json!({ "key": knocking.key, "name": knocking.name })
    } else {
        serde_json::json!({ "key_mark": diverge_desktop_room::key_mark(&knock.space.id, &knocking.key), "name": knocking.name, "listed": false })
    };
    let admitted = call_as_you(state, &knock.space, "admit", args).await;
    match admitted {
        Ok(r) if r.is_error != Some(true) => {
            state.spaces.answer(knock_id, spaces::Answer::Authorized).await?;
            Ok(())
        }
        Ok(r) => {
            state.spaces.answer(knock_id, spaces::Answer::Denied).await?;
            Err(text_of(&r))
        }
        Err(e) => {
            state.spaces.answer(knock_id, spaces::Answer::Denied).await?;
            Err(e)
        }
    }
}

/// One ask, sent to several rooms at once: the same thread in each, so
/// Home follows it everywhere. It goes only to rooms you're in.
#[tauri::command]
pub async fn asks_send(state: State<'_, AppState>, what: String, needs: Option<String>, ceiling: Option<String>, rooms: Vec<String>) -> Result<Vec<AskSent>, String> {
    send_ask(&state, what, needs, ceiling, rooms).await
}

async fn send_ask(state: &AppState, what: String, needs: Option<String>, ceiling: Option<String>, rooms: Vec<String>) -> Result<Vec<AskSent>, String> {
    if what.trim().is_empty() {
        return Err("an ask needs words".into());
    }
    // A thread id of its own in each room: the same words in two rooms don't
    // say they came from one person, unless the words do.
    let base = diverge_desktop_room::fresh_label();
    let mut out = Vec::new();
    for room in rooms {
        let thread = diverge_desktop_room::seal::digest(format!("{base}\n{room}").as_bytes())[..12].to_owned();
        {
            let mut threads = state.threads.lock().unwrap();
            threads.insert(thread.clone(), base.clone());
            state.keep(THREADS_FILE, crate::store::THREADS, &*threads);
        }
        let args = serde_json::json!({ "what": what.trim(), "needs": needs, "ceiling": ceiling, "who_may_serve": "anyone", "thread": thread });
        let outcome = match call_as_you(state, &space_id(&room), "ask", args).await {
            Ok(r) if r.is_error != Some(true) => CallOutcome::Ok { text: text_of(&r) },
            Ok(r) => CallOutcome::Error { message: text_of(&r) },
            Err(message) => CallOutcome::Error { message },
        };
        out.push(AskSent { room, outcome });
    }
    Ok(out)
}

/// Close one of your asks in every room it went to that still has it open:
/// the people who offered see it's closed.
#[tauri::command]
pub async fn asks_close(state: State<'_, AppState>, thread: String, note: Option<String>) -> Result<Vec<AskSent>, String> {
    close_ask(&state, thread, note).await
}

async fn close_ask(state: &AppState, thread: String, note: Option<String>) -> Result<Vec<AskSent>, String> {
    let rooms: Vec<String> = {
        let threads = state.threads.lock().unwrap();
        threads.iter().filter(|(_, base)| **base == thread).map(|(room_thread, _)| room_thread.clone()).collect()
    };
    let mut out = Vec::new();
    for e in state.spaces.list().await {
        let Some(moves) = read_json::<Vec<MoveView>>(state, &e.id, diverge_desktop_room::room::FEED).await else { continue };
        for m in moves.iter().filter(|m| m.kind == "ask" && m.state == "open") {
            let t = m.fields.get("thread").and_then(serde_json::Value::as_str).unwrap_or_default();
            if !rooms.iter().any(|r| r == t) {
                continue;
            }
            let outcome = match call_as_you(state, &e.id, "close_ask", serde_json::json!({ "ask_id": m.id, "note": note })).await {
                Ok(r) if r.is_error != Some(true) => CallOutcome::Ok { text: text_of(&r) },
                Ok(r) => CallOutcome::Error { message: text_of(&r) },
                Err(message) => CallOutcome::Error { message },
            };
            out.push(AskSent { room: e.id.id.clone(), outcome });
        }
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

/// Your agent's allowance in a room, for one kind of move: your setting, nobody else's.
#[tauri::command]
pub fn allowance_set(state: State<'_, AppState>, id: String, agent: String, reach: crate::door::Reach, per_day: u32) -> AllowanceView {
    state.door.set_allowance(&id, &agent, reach, per_day);
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
    rename_machine(&state, &identity, &name)
}

fn rename_machine(state: &AppState, identity: &ProviderView, name: &str) -> HashMap<String, String> {
    let mut names = state.machine_names.lock().unwrap();
    let key = identity_key(identity);
    if name.trim().is_empty() {
        names.remove(&key);
    } else {
        names.insert(key, name.trim().to_owned());
    }
    state.keep(MACHINE_NAMES_FILE, crate::store::MACHINE_NAMES, &*names);
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
pub fn views_save(state: State<'_, AppState>, view: SavedView) -> Result<SavedView, String> {
    save_view(&state, view)
}

fn save_view(state: &AppState, mut view: SavedView) -> Result<SavedView, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    use async_trait::async_trait;
    use chrono::Utc;
    use futures::stream;
    use rmcp::model::{CallToolRequestParams, CallToolResult, ErrorData, ListToolsResult, ReadResourceResult, ServerNotification};
    use serde_json::{Value, json};

    use diverge_desktop_room::{Args, Keypair, Record, Room, Statement, room as program};
    use diverge_sdk::shared::error::Error as WireError;
    use diverge_sdk::shared::filetree::response::Node;

    use crate::daemon::Frames;
    use crate::identity::Identity as Keys;
    use crate::spaces::{Answer, Authorize, Container, HostCall, Id, Invite, Joined, Knock, Knocking, SpaceEntry};
    use crate::store;

    fn wire(message: impl Into<String>) -> WireError {
        WireError(json!({ "message": message.into() }))
    }

    /// Rooms run in process by the real room program, with every call into
    /// a room and every answer at a door written down in order.
    struct Rooms {
        me: Arc<Keys>,
        rooms: Mutex<IndexMap<String, Room>>,
        offline: Mutex<HashSet<String>>,
        knocks: Mutex<Vec<Knock>>,
        log: Mutex<Vec<String>>,
    }

    /// A room's host's side: receipts sealed as its host.
    struct HostSide<'a> {
        me: &'a Keys,
        host_key: String,
    }

    impl diverge_desktop_room::Host for HostSide<'_> {
        fn seal(&self, kind: &str, body: Value) -> Result<Statement, String> {
            self.me.state(&self.host_key, kind, body)
        }
    }

    impl Rooms {
        fn new(me: Arc<Keys>) -> Arc<Rooms> {
            Arc::new(Rooms { me, rooms: Mutex::default(), offline: Mutex::default(), knocks: Mutex::default(), log: Mutex::default() })
        }

        fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }

        fn note(&self, what: String) {
            self.log.lock().unwrap().push(what);
        }

        fn no_room() -> ErrorData {
            ErrorData::invalid_request("no such room", None)
        }
    }

    #[async_trait]
    impl Spaces for Rooms {
        async fn list(&self) -> Vec<SpaceEntry> {
            let offline = self.offline.lock().unwrap().clone();
            self.rooms
                .lock()
                .unwrap()
                .iter()
                .map(|(id, r)| SpaceEntry {
                    id: Id { id: id.clone() },
                    title: r.args.title.clone(),
                    kind: r.args.kind.key().into(),
                    host: Identity::Outgoing { address: "127.0.0.1:4640".into() },
                    host_name: r.args.host_name.clone(),
                    host_key: r.args.host_key.clone(),
                    mine: true,
                    online: !offline.contains(id),
                })
                .collect()
        }

        async fn host(&self, container: Container) -> Result<Id, WireError> {
            let mut arguments = container.arguments;
            let o = arguments.as_object_mut().ok_or_else(|| wire("a room's settings are an object"))?;
            let secret = o.remove("room_secret").and_then(|v| v.as_str().map(str::to_owned)).ok_or_else(|| wire("a room needs its key"))?;
            let key = Keypair::from_secret_hex(&secret).map_err(wire)?;
            let args: Args = serde_json::from_value(arguments).map_err(|e| wire(e.to_string()))?;
            let room = Room::from_record(Record { args, before: None, moves: Vec::new() }, Some(key)).map_err(wire)?;
            let id = room.id().to_owned();
            self.rooms.lock().unwrap().insert(id.clone(), room);
            Ok(Id { id })
        }

        fn knocks(&self, _: CancellationToken) -> Frames<Knock> {
            Box::pin(stream::iter(self.knocks.lock().unwrap().clone()))
        }

        async fn answer(&self, knock_id: u64, answer: Answer) -> Result<Knock, String> {
            let said = match answer {
                Answer::Authorized => "yes",
                Answer::Denied => "no",
            };
            self.note(format!("answer {knock_id} {said}"));
            let mut knocks = self.knocks.lock().unwrap();
            let at = knocks.iter().position(|k| k.knock_id == knock_id).ok_or("nobody is at that door")?;
            Ok(knocks.remove(at))
        }

        async fn pending(&self, knock_id: u64) -> Option<Knock> {
            self.knocks.lock().unwrap().iter().find(|k| k.knock_id == knock_id).cloned()
        }

        async fn join(&self, _: &Invite, _: &Knocking) -> Joined {
            Joined::Missing
        }

        async fn leave(&self, id: &Id) -> Result<(), String> {
            self.rooms.lock().unwrap().shift_remove(&id.id);
            Ok(())
        }

        async fn tools(&self, id: &Id) -> Result<ListToolsResult, ErrorData> {
            self.rooms.lock().unwrap().get(&id.id).map(Room::tools).ok_or_else(Self::no_room)
        }

        async fn read(&self, id: &Id, uri: &str) -> Result<ReadResourceResult, ErrorData> {
            if self.offline.lock().unwrap().contains(&id.id) {
                return Err(ErrorData::internal_error("the room's host is offline", None));
            }
            self.rooms.lock().unwrap().get(&id.id).ok_or_else(Self::no_room)?.read(uri)
        }

        async fn call(&self, id: &Id, params: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
            self.note(format!("call {}", params.name));
            let mut rooms = self.rooms.lock().unwrap();
            let room = rooms.get_mut(&id.id).ok_or_else(Self::no_room)?;
            let host = HostSide { me: &self.me, host_key: room.args.host_key.clone() };
            room.call_at(params, Utc::now(), &host)
        }

        fn notifications(&self, _: &Id, _: CancellationToken) -> Frames<ServerNotification> {
            Box::pin(stream::empty())
        }

        async fn invite(&self, _: &Id) -> Option<Invite> {
            None
        }

        async fn home(&self) -> Option<Id> {
            None
        }

        async fn profile(&self) -> Option<Id> {
            None
        }

        fn host_calls(&self, _: CancellationToken) -> Frames<HostCall> {
            Box::pin(stream::empty())
        }

        async fn table_tree(&self, _: &Id) -> Result<Vec<Node>, WireError> {
            Err(wire("no tables here"))
        }

        async fn table_read(&self, _: &Id, _: &[String]) -> Result<Vec<u8>, WireError> {
            Err(wire("no tables here"))
        }

        async fn table_write(&self, _: &Id, _: &[String], _: Vec<u8>) -> Result<(), WireError> {
            Err(wire("no tables here"))
        }

        async fn transfer(&self, _: &Id, _: &[String], _: &Id) -> Result<(), WireError> {
            Err(wire("no tables here"))
        }

        async fn restart(&self, _: &Id) -> Result<(), WireError> {
            Err(wire("not here"))
        }
    }

    /// The app over a folder of its own, with rooms in process and no daemon.
    fn app_in(data: PathBuf) -> (AppState, Arc<Rooms>) {
        let identity = Arc::new(named(&data, "maya"));
        let rooms = Rooms::new(identity.clone());
        let absent = Arc::new(crate::absent::Absent::not_yet());
        let seams = Seams { daemon: absent.clone(), machines: absent, spaces: rooms.clone(), stand_in_host: None, network: true, first_mounts: HashMap::new() };
        let folder = store::hold(&data);
        (AppState::assemble(data, identity, seams, folder), rooms)
    }

    fn app(what: &str) -> (AppState, Arc<Rooms>) {
        app_in(store::tests::folder(what))
    }

    /// Keys made from a fixed seed, so the stand-in's rooms get the ids
    /// every other test's do, with the first-run page finished.
    fn seeded_keys(data: &Path) {
        let keys = json!({ "personas": [{ "id": "usual", "name": "maya", "secret": Keypair::from_seed("maya").secret_hex(), "created": Utc::now(), "usual": true }], "agents": {}, "rooms": {} });
        store::save(&data.join(KEYS_FILE), store::KEYS, &keys).unwrap();
        Keys::open(data.join(KEYS_FILE)).finish_first_run("maya", true).unwrap();
    }

    /// Your keys in a folder, with the first-run page finished under `name`.
    fn named(data: &Path, name: &str) -> Keys {
        let keys = Keys::open(data.join(KEYS_FILE));
        keys.finish_first_run(name, true).unwrap();
        keys
    }

    async fn board(state: &AppState, title: &str) -> String {
        match host_space(state, HostSpaceInput { title: title.into(), kind: "board".into(), charter: "Be kind about other people's work.".into(), open_door: false }).await.unwrap() {
            HostOutcome::Hosted { id } => id,
            HostOutcome::Error { message } => panic!("{message}"),
        }
    }

    fn knock(knock_id: u64, room: &str, who: &Keypair, name: &str, invite: Option<&str>) -> Knock {
        let knocking = Knocking {
            room: room.into(),
            invite: invite.map(|secret| Knocking::invite_mark(room, secret)),
            key: who.key(),
            name: name.into(),
            note: "I fix lamps and radios.".into(),
            listed: true,
            vouch: None,
            at: Utc::now(),
            sig: String::new(),
        }
        .signed(who);
        Knock { knock_id, space: Id { id: room.into() }, authorize: Authorize { address: "10.0.0.42".parse().unwrap(), authorization: knocking.to_authorization() }, at: Utc::now() }
    }

    async fn asks_in(state: &AppState, room: &str) -> Vec<MoveView> {
        read_json::<Vec<MoveView>>(state, &space_id(room), program::FEED).await.unwrap().into_iter().filter(|m| m.kind == "ask").collect()
    }

    #[test]
    fn a_knock_with_another_invite_is_not_invited() {
        let ren = Keypair::from_seed("ren");
        let k = knock(1, "room-1", &ren, "ren", Some("the room's secret"));
        let wrong = knock_view_of(&k, "Saturday Workshop".into(), Some("another secret"), &[], Utc::now());
        assert!(wrong.checked, "signed by the key it names, for this room, just now");
        assert!(!wrong.invited, "but not with this room's invite");
        assert!(knock_view_of(&k, "Saturday Workshop".into(), Some("the room's secret"), &[], Utc::now()).invited);
        assert!(!knock_view_of(&k, "Saturday Workshop".into(), None, &[], Utc::now()).invited, "a room with no invite");
    }

    #[tokio::test]
    async fn letting_someone_in_admits_them_before_the_door_says_yes() {
        let (state, rooms) = app("actions-knock");
        let id = board(&state, "Saturday Workshop").await;
        let ren = Keypair::from_seed("ren");
        rooms.knocks.lock().unwrap().push(knock(7, &id, &ren, "ren", None));
        answer_knock(&state, 7, true).await.unwrap();
        assert_eq!(rooms.log(), ["call admit", "answer 7 yes"], "the room lets them in, then the door opens");
        assert!(rooms.rooms.lock().unwrap()[&id].member(&ren.key()).is_some_and(|m| !m.removed));
        // A knock made for another room is turned away, and nobody is let in.
        let ada = Keypair::from_seed("ada");
        let elsewhere = Knock { space: Id { id: id.clone() }, ..knock(8, "another room", &ada, "ada", None) };
        rooms.knocks.lock().unwrap().push(elsewhere);
        assert!(answer_knock(&state, 8, true).await.is_err());
        assert_eq!(rooms.log()[2..], ["answer 8 no"]);
        assert!(rooms.rooms.lock().unwrap()[&id].member(&ada.key()).is_none());
    }

    #[tokio::test]
    async fn one_ask_in_two_rooms_is_one_thread_and_closes_in_both() {
        let (state, _) = app("actions-asks");
        let (a, b) = (board(&state, "Saturday Workshop").await, board(&state, "Tuesday repair café").await);
        let sent = send_ask(&state, "A ladder for Saturday".into(), Some("three metres".into()), None, vec![a.clone(), b.clone()]).await.unwrap();
        assert!(sent.iter().all(|s| matches!(s.outcome, CallOutcome::Ok { .. })), "{sent:?}");
        let threads = state.threads.lock().unwrap().clone();
        let bases: HashSet<&String> = threads.values().collect();
        assert_eq!((threads.len(), bases.len()), (2, 1), "a thread id in each room, one thread");
        let base = bases.into_iter().next().unwrap().clone();
        let (in_a, in_b) = (asks_in(&state, &a).await, asks_in(&state, &b).await);
        for asks in [&in_a, &in_b] {
            assert_eq!(asks.len(), 1);
            assert_eq!(asks[0].state, "open");
            assert!(threads.contains_key(asks[0].fields["thread"].as_str().unwrap()));
        }
        assert_ne!(in_a[0].fields["thread"], in_b[0].fields["thread"], "the rooms can't tell it's one ask");
        let closed = close_ask(&state, base, Some("found one".into())).await.unwrap();
        assert_eq!(closed.iter().filter(|c| matches!(c.outcome, CallOutcome::Ok { .. })).count(), 2, "{closed:?}");
        for room in [&a, &b] {
            assert_eq!(asks_in(&state, room).await[0].state, "closed");
        }
        assert_eq!(store::header(&state.data.join(THREADS_FILE)).map(|h| h.version), Some(store::THREADS.version), "the threads file says its version");
    }

    #[tokio::test]
    async fn a_copy_cut_short_is_set_aside_and_the_one_before_it_still_replays() {
        let (state, rooms) = app("actions-copy");
        let id = board(&state, "Saturday Workshop").await;
        let sid = space_id(&id);
        call_as_you(&state, &sid, "show", json!({ "title": "a shelf I built" })).await.unwrap();
        let (first, _) = moves_of(&state, &sid).await;
        call_as_you(&state, &sid, "show", json!({ "title": "and a stool" })).await.unwrap();
        let (second, _) = moves_of(&state, &sid).await;
        assert_eq!(second.len(), first.len() + 1);
        let file = record_file(&state, &id);
        let whole = std::fs::read(&file).unwrap();
        let cut = &whole[..whole.len() / 3];
        std::fs::write(&file, cut).unwrap();
        rooms.offline.lock().unwrap().insert(id.clone());
        let (moves, from_copy) = moves_of(&state, &sid).await;
        assert!(from_copy, "the room can't be reached, so your copy is read");
        assert_eq!(moves.len(), first.len(), "the copy before the damaged one");
        assert!(copy_room(&state, &id).is_some(), "and it replays");
        let aside = file_notices(&state).await;
        assert_eq!(aside.len(), 1, "{aside:?}");
        let n = &aside[0];
        assert_eq!((n.file.clone(), n.why.clone(), n.kind.as_str()), (file.display().to_string(), FileWhy::Damaged, "record copy"));
        assert_eq!(n.room.as_deref(), Some("Saturday Workshop"), "named for its room, not its file");
        assert_eq!((n.carried_on.clone(), n.last_good_copy), (CarriedOnView::LastGood, false));
        assert_eq!(std::fs::read(n.kept_as.as_ref().unwrap()).unwrap(), cut, "kept exactly as it was");
    }

    #[tokio::test]
    async fn a_copy_that_parses_but_isnt_its_rooms_is_refused_not_called_unreadable() {
        let (state, rooms) = app("actions-copy-refused");
        let (a, b) = (board(&state, "Saturday Workshop").await, board(&state, "Tuesday repair café").await);
        for room in [&a, &b] {
            call_as_you(&state, &space_id(room), "show", json!({ "title": "a shelf I built" })).await.unwrap();
            moves_of(&state, &space_id(room)).await;
        }
        // Only one copy each, so nothing to carry on from.
        let _ = std::fs::remove_file(crate::store::previous_of(&record_file(&state, &b)));
        std::fs::copy(record_file(&state, &a), record_file(&state, &b)).unwrap();
        rooms.offline.lock().unwrap().insert(b.clone());
        let (moves, from_copy) = moves_of(&state, &space_id(&b)).await;
        assert!(!from_copy && moves.is_empty(), "another room's record is never shown as this one");
        let aside = file_notices(&state).await;
        assert_eq!(aside.len(), 1, "{aside:?}");
        assert_eq!((aside[0].why.clone(), aside[0].carried_on.clone()), (FileWhy::Refused, CarriedOnView::Empty));
        assert_eq!(aside[0].room.as_deref(), Some("Tuesday repair café"));
    }

    #[tokio::test]
    async fn views_from_a_newer_version_are_set_aside_and_never_written_over() {
        let data = store::tests::folder("actions-newer");
        let newer = r#"{"file":"views","version":99,"data":"a shape this build doesn't know"}"#;
        std::fs::write(data.join(VIEWS_FILE), newer).unwrap();
        let (state, _) = app_in(data);
        assert!(state.views.lock().unwrap().is_empty());
        let aside = file_notices(&state).await;
        assert_eq!(aside.len(), 1, "{aside:?}");
        assert_eq!((aside[0].why.clone(), aside[0].kind.as_str(), aside[0].room.clone()), (FileWhy::Newer, "views", None));
        let view: SavedView = serde_json::from_value(json!({ "id": "", "title": "What it said", "query": { "name": "site-fixes", "logs_index_from": null, "logs_index_to": null, "created_from": null, "created_to": null, "item_type": null, "jq": null, "count": null, "watch": false }, "saved": "" })).unwrap();
        save_view(&state, view).unwrap();
        assert_eq!(std::fs::read_to_string(aside[0].kept_as.as_ref().unwrap()).unwrap(), newer, "untouched");
        assert_eq!(store::header(&state.data.join(VIEWS_FILE)).map(|h| h.version), Some(store::VIEWS.version));
    }

    #[tokio::test]
    async fn with_nothing_answering_the_app_says_the_network_part_isnt_there() {
        let data = store::tests::folder("actions-absent");
        let identity = Arc::new(named(&data, "maya"));
        let absent = Arc::new(crate::absent::Absent::not_yet());
        let seams = Seams { daemon: absent.clone(), machines: absent.clone(), spaces: absent, stand_in_host: None, network: false, first_mounts: HashMap::new() };
        let folder = store::hold(&data);
        let state = AppState::assemble(data, identity, seams, folder);
        let about = info(&state);
        assert!(!about.network && !about.stand_in);
        assert!(listed(state.daemon.agents_list(agents::list::client::request::Frame {}).collect().await).agents.is_empty(), "no agents made up");
        assert!(state.daemon.providers_list().await.is_empty(), "no machines made up");
        assert!(state.spaces.list().await.is_empty(), "no rooms made up");
        match host_space(&state, HostSpaceInput { title: "Saturday Workshop".into(), kind: "board".into(), charter: String::new(), open_door: false }).await.unwrap() {
            HostOutcome::Error { message } => assert_eq!(message, crate::absent::NOT_YET),
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn with_the_data_dir_variable_set_the_app_keeps_its_files_there() {
        let dir = store::tests::folder("actions-data-dir");
        let data = data_dir::<std::io::Error>(Some(dir.clone().into_os_string()), || unreachable!("not the system's folder")).unwrap();
        assert_eq!(data, dir);
        assert_eq!(data_dir::<()>(Some("".into()), || Ok(PathBuf::from("/the/system/folder"))), Ok(PathBuf::from("/the/system/folder")), "an empty one names nothing");
        assert_eq!(data_dir::<()>(Some("second-copy".into()), || unreachable!()), Ok(std::env::current_dir().unwrap().join("second-copy")), "a relative one is from where the app started");
        // The stand-in's rooms need the keys every other test has; without it, the app makes its own.
        #[cfg(feature = "stand-in")]
        seeded_keys(&data);
        let state = AppState::open(data.clone()).await;
        assert_eq!(state.network, cfg!(feature = "stand-in"));
        assert!(state.folder.held());
        #[cfg(not(feature = "stand-in"))]
        {
            assert!(!data.join(KEYS_FILE).exists(), "no keys until you've said what to call you");
            finish_first_run(&state, "Ada", true).unwrap();
            assert_eq!(store::header(&data.join(KEYS_FILE)).map(|h| (h.file, h.version)), Some(("keys".into(), store::KEYS.version)), "your keys, made and kept there");
        }
        save_view(&state, a_view()).unwrap();
        assert_eq!(store::header(&data.join(VIEWS_FILE)).map(|h| (h.file, h.version)), Some(("views".into(), 1)), "what you keep, kept there");
        assert!(data.join(RECORDS_DIR).is_dir());
        #[cfg(feature = "stand-in")]
        assert!(std::fs::read_dir(data.join(RECORDS_DIR)).unwrap().next().is_some(), "the stand-in's copies are there too");
    }

    fn a_view() -> SavedView {
        serde_json::from_value(json!({ "id": "", "title": "What it said", "query": { "name": "site-fixes", "logs_index_from": null, "logs_index_to": null, "created_from": null, "created_to": null, "item_type": null, "jq": null, "count": null, "watch": false }, "saved": "" })).unwrap()
    }

    /// Every file under a folder, and what it holds.
    fn snapshot(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        let mut files = Vec::new();
        walk(dir, &mut files);
        files.into_iter().map(|f| { let bytes = std::fs::read(&f).unwrap(); (f, bytes) }).collect()
    }

    #[tokio::test]
    async fn a_second_copy_on_a_folder_in_use_changes_nothing_and_says_so() {
        let data = store::tests::folder("actions-in-use");
        seeded_keys(&data);
        store::save(&data.join(VIEWS_FILE), store::VIEWS, &vec![a_view()]).unwrap();
        store::save(&data.join(THREADS_FILE), store::THREADS, &HashMap::from([("t-1".to_owned(), "t".to_owned())])).unwrap();
        std::fs::write(data.join(MACHINE_NAMES_FILE), "{ damaged").unwrap();
        // Another copy of the app, holding the folder.
        let other = store::hold(&data);
        assert!(other.held());
        let before = snapshot(&data);
        let state = AppState::open(data.clone()).await;
        assert_eq!(info(&state).folder_held, FolderHeld::InUse);
        assert!(!state.network);
        assert_eq!(state.identity.usual().unwrap().key, Keypair::from_seed("maya").key(), "your keys, read as they are");
        assert_eq!(state.views.lock().unwrap().len(), 1, "your files, read as they are");
        // Whatever it's asked to do, it changes nothing there.
        save_view(&state, a_view()).unwrap();
        rename_machine(&state, &(&Identity::Outgoing { address: "127.0.0.1:4640".into() }).into(), "the desk upstairs");
        state.door.set_allowance("room-1", "site-fixes", crate::door::Reach::Talk, 2);
        state.identity.fresh("lamp person").ok();
        let mut params = rmcp::model::CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        assert_eq!(state.identity.seal(&crate::identity::Actor::Persona("usual".into()), "room-1", &mut params).unwrap_err(), store::IN_USE);
        match host_space(&state, HostSpaceInput { title: "Saturday Workshop".into(), kind: "board".into(), charter: String::new(), open_door: false }).await {
            Ok(HostOutcome::Error { message }) | Err(message) => assert_eq!(message, store::IN_USE, "and says why"),
            other => panic!("{other:?}"),
        }
        assert!(moves_of(&state, &space_id("room-1")).await.0.is_empty());
        assert_eq!(snapshot(&data), before, "not one file written, moved or made");
        assert!(file_notices(&state).await.is_empty(), "nothing set aside: the damaged file is the other copy's to deal with");
        drop(state);
        drop(other);
        // Once the other copy closes, the folder is this one's.
        let again = AppState::open(data.clone()).await;
        assert_eq!(info(&again).folder_held, FolderHeld::Yes);
        assert_eq!(again.identity.usual().unwrap().key, Keypair::from_seed("maya").key());
    }

    #[tokio::test]
    async fn two_copies_opened_on_one_folder_and_the_second_finds_it_in_use() {
        let data = store::tests::folder("actions-two-copies");
        seeded_keys(&data);
        let first = AppState::open(data.clone()).await;
        let second = AppState::open(data.clone()).await;
        assert_eq!((info(&first).folder_held, info(&second).folder_held), (FolderHeld::Yes, FolderHeld::InUse));
        assert!(second.identity.broken().is_none(), "its keys are fine; the folder is the other copy's");
    }

    /// The stand-in's own words are in the program only in a build that asked for it.
    #[test]
    fn the_stand_in_is_built_in_only_with_its_feature() {
        let program = String::from_utf8_lossy(&std::fs::read(std::env::current_exe().unwrap()).unwrap()).into_owned();
        let looked_for = ["diverge-desktop|stand-in|daemon|is|built|in", "diverge-desktop|stand-in|rooms|are|built|in"].map(|w| w.replace('|', " "));
        #[cfg(feature = "stand-in")]
        assert_eq!([crate::daemon::stub::BUILT_IN, crate::spaces::stub::BUILT_IN], [looked_for[0].as_str(), looked_for[1].as_str()]);
        for words in &looked_for {
            assert_eq!(program.contains(words.as_str()), cfg!(feature = "stand-in"), "{words}");
        }
    }

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { walk(&path, out) } else { out.push(path) }
        }
    }

    #[cfg(feature = "stand-in")]
    #[tokio::test]
    async fn every_file_a_stand_in_run_keeps_carries_its_version() {
        let data = store::tests::folder("actions-full-run");
        seeded_keys(&data);
        let state = AppState::open(data.clone()).await;
        // Everything that keeps a file, once.
        state.remember_mounts("site-fixes", Some(AgentMounts::default()));
        save_view(&state, a_view()).unwrap();
        let machine = state.daemon.providers_list().await.into_iter().next().expect("the stand-in has a machine");
        rename_machine(&state, &(&machine.identity).into(), "the desk upstairs");
        let entries = state.spaces.list().await;
        let mine: Vec<String> = entries.iter().filter(|e| e.mine && (e.kind == "home" || e.kind == "board")).map(|e| e.id.id.clone()).collect();
        assert_eq!(mine.len(), 2, "your home and your board");
        state.door.set_allowance(&mine[0], "site-fixes", crate::door::Reach::Talk, 2);
        for round in ["A ladder for Saturday", "A long extension cable"] {
            send_ask(&state, round.into(), None, None, mine.clone()).await.unwrap();
            for e in &entries {
                moves_of(&state, &e.id).await;
            }
        }
        let mut files = Vec::new();
        walk(&data, &mut files);
        let mut kinds = HashSet::new();
        for file in files {
            let parts: Vec<String> = file.strip_prefix(&data).unwrap().components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            // A stand-in machine's volumes and a room's table hold their own files, not the app's.
            if parts[0] == "stand-in-host" && ((parts[1] == "machines" && parts.len() > 4 && parts[3] == "volumes") || (parts[1] == "tables" && parts.len() > 3)) {
                continue;
            }
            let name = parts.last().unwrap();
            if name.starts_with('.') && name.ends_with(".tmp") {
                continue; // a write still under way when the folder was read
            }
            if parts.len() == 1 && name == store::IN_USE_FILE {
                assert_eq!(std::fs::metadata(&file).unwrap().len(), 0, "holds the folder, says nothing");
                continue;
            }
            let header = store::header(&file).unwrap_or_else(|| panic!("{} carries no version", parts.join("/")));
            kinds.insert(header.file);
        }
        for kind in ["keys", "counters", "allowances", "views", "machine names", "agent mounts", "threads", "record copy", "stand-in rooms", "stand-in volumes"] {
            assert!(kinds.contains(kind), "a {kind} file was kept: {kinds:?}");
        }
        assert!(file_notices(&state).await.is_empty(), "nothing set aside on a clean run");
    }

    #[tokio::test]
    async fn from_an_empty_folder_nobody_is_named_and_nothing_is_signed_until_the_first_run_page_is_finished() {
        // Run under a known login: it's there to be read, and the app never reads it.
        if !store::tests::under_a_known_login(module_path!(), "from_an_empty_folder_nobody_is_named_and_nothing_is_signed_until_the_first_run_page_is_finished") {
            return;
        }
        let data = store::tests::folder("actions-first-run");
        let state = AppState::open(data.clone()).await;
        assert_eq!(FirstRunView::from(state.identity.first_run()), FirstRunView::New);
        assert!(state.identity.personas().is_empty(), "no persona");
        assert!(state.spaces.list().await.is_empty(), "no rooms: the stand-in's are yours too, so they wait for you");
        match host_space(&state, HostSpaceInput { title: "Saturday Workshop".into(), kind: "board".into(), charter: String::new(), open_door: false }).await {
            Err(message) => assert_eq!(message, crate::identity::NOT_NAMED),
            #[cfg(not(feature = "stand-in"))]
            Ok(HostOutcome::Error { .. }) => {}
            other => panic!("{other:?}"),
        }
        let invite = spaces::Invite { host: Identity::Outgoing { address: "127.0.0.1:4640".into() }, id: "workshop.abc".into(), secret: Some("s".into()), title: "Workshop".into(), kind: "board".into(), host_name: "ren".into(), charter: String::new(), verbs: Vec::new() }.to_text();
        assert_eq!(door_of(&state, &invite).await.unwrap_err(), crate::identity::NOT_NAMED, "no name for a knock to send");
        assert_eq!(finish_first_run(&state, "Ada", false).unwrap_err(), crate::identity::NOT_ADULT);
        assert!(!data.join(KEYS_FILE).exists(), "no keys made");
        assert!(!data.join("stand-in-host").join("tables").join(".stand-in-rooms.json").exists(), "nothing seeded");
        assert!(std::fs::read_dir(data.join(RECORDS_DIR)).unwrap().next().is_none(), "no copies of anything");
        // The page, finished.
        assert_eq!(finish_first_run(&state, "Ada", true).unwrap(), FirstRunView::Done);
        assert_eq!(door_of(&state, &invite).await.unwrap().usual_name, "Ada", "the door shows the exact name a knock sends");
        #[cfg(feature = "stand-in")]
        {
            let entries = state.spaces.list().await;
            let home = entries.iter().find(|e| e.mine && e.kind == "home").expect("the stand-in's rooms, now there's a you");
            assert_eq!(home.host_name, "Ada", "under the name you typed");
            assert!(std::fs::read_dir(data.join(RECORDS_DIR)).unwrap().next().is_some(), "and the stand-in's copies");
        }
        // Everything the app keeps, after the first run: the recovery words
        // only sealed, and only in your keys file; never the words in plain,
        // the seed they make, the root or its chain code; never the login.
        let words = state.identity.words().expect("an account, and its words");
        let seed = bip39::Mnemonic::parse(words.as_str()).unwrap().to_seed("");
        let root = diverge_desktop_room::account::derive(&seed, diverge_desktop_room::account::ROOT_PATH).unwrap();
        let sealed = state.identity.sealed_words().expect("the words, sealed");
        let never: Vec<(&str, Vec<u8>)> = vec![
            ("the words", words.as_bytes().to_vec()),
            ("the words run together", words.replace(' ', "").into_bytes()),
            ("the seed", seed.to_vec()),
            ("the seed", hex::encode(seed).into_bytes()),
            ("the root's secret", root.key.to_vec()),
            ("the root's secret", hex::encode(root.key).into_bytes()),
            ("the root's secret", diverge_desktop_room::account::root(&seed).secret_hex().into_bytes()),
            ("the root's chain code", root.chain.to_vec()),
            ("the root's chain code", hex::encode(root.chain).into_bytes()),
            ("the login", store::tests::LOGIN.as_bytes().to_vec()),
        ];
        let holds = |bytes: &[u8], needle: &[u8]| !needle.is_empty() && bytes.windows(needle.len()).any(|w| w == needle);
        let keys = data.join(KEYS_FILE);
        let keys_backup = store::previous_of(&keys);
        let mut files = Vec::new();
        walk(&data, &mut files);
        // Without the stand-in, your keys file and the folder's lock; with it, its rooms and their copies too.
        assert!(files.len() > if cfg!(feature = "stand-in") { 5 } else { 1 }, "the app keeps its files here: {files:?}");
        let mut sealed_in = Vec::new();
        for file in &files {
            // A write under way when the folder was read may be gone by now.
            let Ok(bytes) = std::fs::read(file) else { continue };
            let name = file.strip_prefix(&data).unwrap().display().to_string();
            for (what, needle) in &never {
                assert!(!holds(&bytes, needle), "{name} holds {what}");
            }
            if holds(&bytes, sealed.as_bytes()) {
                sealed_in.push(file.clone());
            }
        }
        assert!(sealed_in.contains(&keys), "your keys file holds the words, sealed");
        for file in &sealed_in {
            // The keys file, its backup, or a new copy of it still being written.
            let name = file.file_name().unwrap().to_string_lossy();
            let keys_being_written = file.parent() == Some(data.as_path()) && name.starts_with(".identity.json.") && name.ends_with(".tmp");
            assert!(*file == keys || *file == keys_backup || keys_being_written, "{} holds the sealed words", file.display());
        }
    }

    /// A folder an earlier version made has keys that may already have
    /// signed and sent as you: nothing more is, until the page is finished.
    #[tokio::test]
    async fn a_folder_from_before_accounts_sends_nothing_more_until_the_first_run_page_is_finished() {
        let data = store::tests::folder("actions-earlier");
        let usual = Keypair::from_seed("sam from before");
        let v1 = json!({ "file": "keys", "version": 1, "data": {
            "personas": [{ "id": "usual", "name": "sam", "secret": usual.secret_hex(), "created": Utc::now(), "usual": true }],
            "agents": {}, "rooms": {}
        } });
        std::fs::write(data.join(KEYS_FILE), v1.to_string()).unwrap();
        let state = AppState::open(data.clone()).await;
        assert_eq!(FirstRunView::from(state.identity.first_run()), FirstRunView::Earlier { name: "sam".into() });
        assert!(state.spaces.list().await.is_empty(), "the stand-in's rooms wait for the page here too");
        match host_space(&state, HostSpaceInput { title: "Saturday Workshop".into(), kind: "board".into(), charter: String::new(), open_door: false }).await {
            Err(message) => assert_eq!(message, crate::identity::NOT_NAMED),
            #[cfg(not(feature = "stand-in"))]
            Ok(HostOutcome::Error { .. }) => {}
            other => panic!("{other:?}"),
        }
        let invite = spaces::Invite { host: Identity::Outgoing { address: "127.0.0.1:4640".into() }, id: "workshop.abc".into(), secret: Some("s".into()), title: "Workshop".into(), kind: "board".into(), host_name: "ren".into(), charter: String::new(), verbs: Vec::new() }.to_text();
        // A knock is signed as you (spaces_join): not yet.
        assert_eq!(state.identity.state(&usual.key(), "knock", json!({ "room": "workshop.abc" })).unwrap_err(), crate::identity::NOT_NAMED, "no knock is signed");
        assert_eq!(state.identity.fresh("lamp person").unwrap_err(), crate::identity::NOT_NAMED, "nor a fresh name to knock as");
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        assert_eq!(state.identity.seal(&crate::identity::Actor::Persona("usual".into()), "room-1", &mut params).unwrap_err(), crate::identity::NOT_NAMED, "and no call to a room is sealed");
        assert_eq!(finish_first_run(&state, "Sam Lee", true).unwrap(), FirstRunView::Done);
        assert_eq!(state.identity.usual().unwrap().key, usual.key(), "the same key as before");
        assert_eq!(door_of(&state, &invite).await.unwrap().usual_name, "Sam Lee", "under the name typed on the page");
    }

    #[tokio::test]
    async fn two_members_of_a_room_with_the_same_name_show_different_marks() {
        let (state, _rooms) = app("actions-marks");
        let id = board(&state, "Saturday Workshop").await;
        let (one, other) = (Keypair::from_seed("one ada"), Keypair::from_seed("another ada"));
        for who in [&one, &other] {
            call_as_you(&state, &space_id(&id), "admit", json!({ "key": who.key(), "name": "ada" })).await.unwrap();
        }
        let view = space_view(&state, id.clone()).await.unwrap();
        let mark = |key: &str| view.members.iter().find(|m| m.key == key).and_then(|m| m.mark.clone());
        let (a, b) = (mark(&one.key()).expect("marked"), mark(&other.key()).expect("marked"));
        assert_ne!(a, b);
        let you = state.identity.usual().unwrap().key;
        assert_eq!(mark(&you), None, "a name nobody else here has needs none");
        let let_in = admitted(&state, &id).await.unwrap();
        assert_eq!(let_in.iter().filter(|p| p.mark.is_some()).map(|p| p.key.clone()).collect::<HashSet<_>>(), HashSet::from([one.key(), other.key()]), "the host's list of everyone let in is marked too");
        assert_eq!(let_in.iter().find(|p| p.key == one.key()).and_then(|p| p.mark.clone()), Some(a), "with the same marks");
    }
}
