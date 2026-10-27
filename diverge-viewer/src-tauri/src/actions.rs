//! The action registry: everything the page asks Rust for that is the
//! app's own — never the daemon's, which the page reaches itself through
//! the SDK over the proxy. Tabs, the theme, saved Views, the names you
//! gave machines, what the app knows of its own folder, and the world's
//! tilesets.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, State};

use crate::store;
use crate::tabs::Tabs;
use crate::tileset;
use crate::view::*;

pub struct AppState {
    /// The folder every file the app keeps is in.
    pub data: PathBuf,
    /// Whether this copy of the app holds that folder. One that doesn't
    /// changes nothing in it: it saves nothing.
    pub folder: store::Hold,
    pub tabs: Mutex<Tabs>,
    pub views: Mutex<Vec<SavedView>>,
    /// Machine names, yours: the key the page gives → name.
    pub machine_names: Mutex<HashMap<String, String>>,
    /// The theme you picked, by its id in theme.css; none until you pick one (the default shows).
    pub theme: Mutex<Option<String>>,
    /// The world folder, `world/` in the app's folder: the tilesets,
    /// read and never written.
    pub world: PathBuf,
    /// The tileset you picked, by its folder's name; none until you pick
    /// one, and [`DEFAULT_TILESET`] shows.
    pub tileset: Mutex<Option<String>>,
}

/// The files the app keeps, in its folder.
const VIEWS_FILE: &str = "views.json";
const MACHINE_NAMES_FILE: &str = "machine_names.json";
const THEME_FILE: &str = "theme.json";
const TILESET_FILE: &str = "tileset.json";

/// The tileset shown until you pick one.
pub const DEFAULT_TILESET: &str = "classic";

/// The app's own folder under the root, beside `viewer.yaml`: every
/// file the app keeps is in it, as the daemon's are in `daemon/`.
pub const FOLDER: &str = "viewer";

impl AppState {
    /// The app over its folder: every file it keeps, read as the folder
    /// is held. One copy of the app at a time uses a folder; a copy that
    /// finds it held by another (or can't tell) reads the files as they
    /// are and changes nothing there, and the screens say why.
    pub fn open(data: PathBuf) -> AppState {
        let folder = store::hold(&data);
        let held = folder.held();
        fn kept<T: serde::de::DeserializeOwned>(held: bool, file: &std::path::Path, format: store::Format) -> Option<T> {
            if held { store::load(file, format) } else { store::peek(file, format) }
        }
        AppState {
            tabs: Mutex::new(Tabs::default()),
            views: Mutex::new(kept(held, &data.join(VIEWS_FILE), store::VIEWS).unwrap_or_default()),
            machine_names: Mutex::new(kept(held, &data.join(MACHINE_NAMES_FILE), store::MACHINE_NAMES).unwrap_or_default()),
            // A theme that's no longer in theme.css falls back to the default.
            theme: Mutex::new(kept::<Option<String>>(held, &data.join(THEME_FILE), store::THEME).flatten().filter(|id| theme_ids().contains(&id.as_str()))),
            tileset: Mutex::new(kept::<Option<String>>(held, &data.join(TILESET_FILE), store::TILESET).flatten()),
            world: data.join(tileset::WORLD),
            data,
            folder,
        }
    }

    /// Keep one of the app's files, as it is now. Nothing is written in a
    /// folder this copy doesn't hold.
    fn keep<T: serde::Serialize + serde::de::DeserializeOwned>(&self, name: &str, format: store::Format, data: &T) {
        if self.folder.held() {
            let _ = store::save(&self.data.join(name), format, data);
        }
    }
}

/// Every action, by name, with what it does: the list `actions_list`
/// answers. The daemon's own verbs are not here — the page performs them
/// itself, with the SDK.
pub const REGISTRY: &[(&str, &str)] = &[
    ("machines_names", "The names you gave your machines"),
    ("machines_rename", "Call a machine by a name of your own"),
    ("theme_get", "The theme you picked (none: the default)"),
    ("theme_set", "Pick a theme by its id, or none for the default"),
    ("tilesets_list", "Every installed tileset: its name, or why it cannot be used"),
    ("tileset_load", "A tileset's manifest, held to every rule"),
    ("tileset_image", "A tileset's atlas image, as its bytes"),
    ("tileset_get", "The tileset you picked, or the default"),
    ("tileset_set", "Pick a tileset by its id, or none for the default"),
    ("views_list", "List saved Views"),
    ("views_save", "Save a View (a saved log request)"),
    ("views_delete", "Delete a saved View"),
    ("tabs_snapshot", "What is open"),
    ("tabs_open", "Open (or focus) a tab"),
    ("tabs_close", "Close a tab"),
    ("actions_list", "This list"),
    ("app_info", "The folder the app keeps its files in, and whether this copy holds it"),
    ("files_set_aside", "Files the app couldn't use since it started: each set aside untouched, or left where it is; what it carried on from"),
];

#[tauri::command]
pub fn actions_list() -> Vec<ActionInfo> {
    REGISTRY.iter().map(|(name, does)| ActionInfo { name: (*name).into(), does: (*does).into() }).collect()
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo { folder: state.data.display().to_string(), folder_held: (&state.folder).into() }
}

/// Files the app couldn't use since it started: each set aside, untouched,
/// under a new name, or left where it is; and what the app carried on from.
#[tauri::command]
pub fn files_set_aside(state: State<'_, AppState>) -> Vec<FileNoticeView> {
    store::notices_under(&state.data).iter().map(FileNoticeView::of).collect()
}

// --- machine names -----------------------------------------------------------

/// The names you gave your machines, by the key the page keeps them under.
#[tauri::command]
pub fn machines_names(state: State<'_, AppState>) -> HashMap<String, String> {
    state.machine_names.lock().unwrap().clone()
}

/// Call a machine by a name of your own; an empty name forgets the one given.
#[tauri::command]
pub fn machines_rename(state: State<'_, AppState>, key: String, name: String) -> HashMap<String, String> {
    let mut names = state.machine_names.lock().unwrap();
    if name.trim().is_empty() {
        names.remove(&key);
    } else {
        names.insert(key, name.trim().to_owned());
    }
    state.keep(MACHINE_NAMES_FILE, store::MACHINE_NAMES, &*names);
    names.clone()
}

// --- the theme -------------------------------------------------------------

/// The themes theme.css holds: the id of each `[data-theme="<id>"]` block, the
/// default's first. Read from the sheet itself, so there is no second list to keep.
pub fn theme_ids() -> Vec<&'static str> {
    const CSS: &str = include_str!("../../src/theme.css");
    CSS.split("[data-theme=\"").skip(1).filter_map(|rest| rest.split_once("\"]").map(|(id, _)| id))
        // Only real ids: the sheet's own comment names the pattern as `<id>`.
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .collect()
}

#[tauri::command]
pub fn theme_get(state: State<'_, AppState>) -> Option<String> {
    state.theme.lock().unwrap().clone()
}

#[tauri::command]
pub fn theme_set(state: State<'_, AppState>, theme: Option<String>) -> Result<Option<String>, String> {
    if let Some(id) = &theme {
        if !theme_ids().contains(&id.as_str()) {
            return Err(format!("there is no theme called {id:?}"));
        }
    }
    let mut kept = state.theme.lock().unwrap();
    *kept = theme;
    state.keep(THEME_FILE, store::THEME, &*kept);
    Ok(kept.clone())
}

// --- the world's tilesets ------------------------------------------------

#[tauri::command]
pub async fn tilesets_list(state: State<'_, AppState>) -> Result<Vec<tileset::TilesetEntry>, String> {
    tileset::list(&state.world).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tileset_load(state: State<'_, AppState>, id: String) -> Result<tileset::Tileset, String> {
    tileset::load(&state.world, &id).await.map_err(|e| e.to_string())
}

/// The atlas image's bytes, raw: the page makes a `blob:` URL of them.
#[tauri::command]
pub async fn tileset_image(state: State<'_, AppState>, id: String) -> Result<tauri::ipc::Response, String> {
    tileset::image(&state.world, &id).await.map(tauri::ipc::Response::new).map_err(|e| e.to_string())
}

/// The tileset you picked, else the default; whether it is installed is
/// `tileset_load`'s to say.
#[tauri::command]
pub fn tileset_get(state: State<'_, AppState>) -> String {
    state.tileset.lock().unwrap().clone().unwrap_or_else(|| DEFAULT_TILESET.to_owned())
}

/// Pick a tileset; one that does not hold up is refused, and the pick is
/// unchanged.
#[tauri::command]
pub async fn tileset_set(state: State<'_, AppState>, tileset: Option<String>) -> Result<String, String> {
    if let Some(id) = &tileset {
        tileset::load(&state.world, id).await.map_err(|e| e.to_string())?;
    }
    let mut kept = state.tileset.lock().unwrap();
    *kept = tileset;
    state.keep(TILESET_FILE, store::TILESET, &*kept);
    Ok(kept.clone().unwrap_or_else(|| DEFAULT_TILESET.to_owned()))
}

// --- saved Views -----------------------------------------------------------

#[tauri::command]
pub fn views_list(state: State<'_, AppState>) -> Vec<SavedView> {
    state.views.lock().unwrap().clone()
}

/// Save a View: a title and a log request, as the page shapes it. A new
/// one gets an id; one with an id is replaced.
#[tauri::command]
pub fn views_save(state: State<'_, AppState>, mut view: SavedView) -> Result<SavedView, String> {
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
    state.keep(VIEWS_FILE, store::VIEWS, &views.to_vec());
    Ok(view)
}

#[tauri::command]
pub fn views_delete(state: State<'_, AppState>, id: String) {
    let mut views = state.views.lock().unwrap();
    views.retain(|v| v.id != id);
    state.keep(VIEWS_FILE, store::VIEWS, &views.to_vec());
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
