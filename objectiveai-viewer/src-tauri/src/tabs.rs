//! The unified tab system's Rust half: the TAB REGISTRY (the single
//! source of truth for which tabs live in which window, and which is
//! active), the Tauri commands the shell frontend drives it with, and
//! the detach flow (tear a tab out into a fresh OS window and hand
//! the user's still-held drag to the OS via `start_dragging`).
//!
//! Every OS window runs the SAME shell entry (`index.html`); a window
//! is just a container of tabs. The registry lives here as managed
//! state; every mutation bumps a `generation` and broadcasts the FULL
//! snapshot as a `tabs://changed` event — each window filters by its
//! own label and rebuilds from truth (events carry no deltas worth
//! trusting; a stale snapshot response can never clobber a newer
//! event because consumers apply payloads only when the generation
//! advances).
//!
//! Locking discipline: the registry is a `std::sync::Mutex`; the
//! guard is NEVER held across an `.await`, a window operation
//! (`set_focus` / `close` / `build`), or an emit — mutate + clone the
//! snapshot under the lock, drop the guard, then act on the clone
//! (the `DaemonProxy.streams` precedent).
//!
//! Docking (the reattach half) lives in [`crate::docking`].

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{Emitter, Manager};

/// The tab strip's height in LOGICAL pixels — the shell's TabStrip is
/// styled to exactly this, and the docking hit-test scales it by the
/// target window's `scale_factor()`.
pub const STRIP_HEIGHT_LOGICAL: f64 = 40.0;

/// What a tab IS — everything needed to render its content anywhere.
/// Serde-tagged; the TS mirror lives in the shell's `App.tsx`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TabKind {
    /// The agents home (hierarchy tree + command palette).
    Agents,
    /// The laboratories home (laboratory builder / list).
    Laboratories,
    /// One agent conversation, by its instance hierarchy.
    Agent { aih: String },
    /// One laboratory browser, by id + optional host pin.
    Laboratory {
        id: String,
        machine: Option<String>,
        machine_state: Option<String>,
        machine_os: Option<String>,
    },
}

impl TabKind {
    /// The tab's display title. Labs keep the `{os}/{machine}/{id}`
    /// format the bespoke window title used (unknown segments `?`).
    fn title(&self) -> String {
        match self {
            TabKind::Agents => "agents".to_string(),
            TabKind::Laboratories => "laboratories".to_string(),
            TabKind::Agent { aih } => aih.clone(),
            TabKind::Laboratory {
                id,
                machine,
                machine_os,
                ..
            } => format!(
                "{}/{}/{id}",
                machine_os.as_deref().unwrap_or("?"),
                machine.as_deref().unwrap_or("?"),
            ),
        }
    }

    /// Whether the shell shows a close button. The two home tabs are
    /// permanent (movable, never closable).
    fn closable(&self) -> bool {
        !matches!(self, TabKind::Agents | TabKind::Laboratories)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Tab {
    pub id: u64,
    // Nested, NOT flattened: `TabKind::Laboratory` has its own `id`
    // field, which flattening would collide with this struct's.
    pub kind: TabKind,
    pub title: String,
    pub closable: bool,
}

/// One window's slice of the registry.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct WindowTabs {
    pub tabs: Vec<Tab>,
    /// The active tab's id (0 = none, only possible on an empty
    /// window).
    pub active: u64,
}

/// The whole registry, as snapshotted to the shells.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Snapshot {
    pub generation: u64,
    pub windows: HashMap<String, WindowTabs>,
}

#[derive(Debug, Default)]
struct Inner {
    windows: HashMap<String, WindowTabs>,
    generation: u64,
    next_tab: u64,
    next_shell: u64,
}

impl Inner {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            generation: self.generation,
            windows: self.windows.clone(),
        }
    }

    /// Find a tab by id: `(window label, index)`.
    fn locate(&self, tab_id: u64) -> Option<(String, usize)> {
        for (label, wt) in &self.windows {
            if let Some(idx) = wt.tabs.iter().position(|t| t.id == tab_id) {
                return Some((label.clone(), idx));
            }
        }
        None
    }

    /// Find a window already holding `kind`: `(label, tab id)`.
    fn locate_kind(&self, kind: &TabKind) -> Option<(String, u64)> {
        for (label, wt) in &self.windows {
            if let Some(tab) = wt.tabs.iter().find(|t| &t.kind == kind) {
                return Some((label.clone(), tab.id));
            }
        }
        None
    }

    fn mint_tab(&mut self, kind: TabKind) -> Tab {
        self.next_tab += 1;
        Tab {
            id: self.next_tab,
            title: kind.title(),
            closable: kind.closable(),
            kind,
        }
    }
}

/// The managed registry. Seeded by `run.rs`'s setup, mutated only
/// through the commands below (+ the docking task and the Destroyed
/// cleanup).
#[derive(Default)]
pub struct TabRegistry {
    inner: Mutex<Inner>,
}

impl TabRegistry {
    /// Seed one window's initial tab set (setup-time; also bumps the
    /// generation so the first snapshot is post-seed).
    pub fn seed(&self, label: &str, kinds: Vec<TabKind>) {
        let mut inner = self.inner.lock().unwrap();
        let tabs: Vec<Tab> = kinds.into_iter().map(|k| inner.mint_tab(k)).collect();
        let active = tabs.first().map(|t| t.id).unwrap_or(0);
        inner
            .windows
            .insert(label.to_string(), WindowTabs { tabs, active });
        inner.generation += 1;
    }

    /// Remove a window's entry entirely (window destroyed). Idempotent.
    /// Returns whether anything changed.
    pub fn remove_window(&self, label: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let removed = inner.windows.remove(label).is_some();
        if removed {
            inner.generation += 1;
        }
        removed
    }

    /// Move every tab of `source` onto the end of `target` (the dock
    /// merge). The moved window's active tab becomes the target's
    /// active. Returns whether the merge happened.
    pub fn merge_windows(&self, source: &str, target: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if !inner.windows.contains_key(target) {
            return false;
        }
        let Some(moved) = inner.windows.remove(source) else {
            return false;
        };
        let target_wt = inner.windows.get_mut(target).expect("checked above");
        let follow_active = moved.active;
        target_wt.tabs.extend(moved.tabs);
        if follow_active != 0 {
            target_wt.active = follow_active;
        }
        inner.generation += 1;
        true
    }

    pub fn snapshot(&self) -> Snapshot {
        self.inner.lock().unwrap().snapshot()
    }
}

/// Broadcast the snapshot to every window + retitle `touched` windows.
/// Call with NO registry guard held.
fn publish(app: &tauri::AppHandle, snapshot: &Snapshot, touched: &[String]) {
    // Best-effort everywhere: a window mid-teardown must never turn a
    // mutation into an error.
    let _ = app.emit("tabs://changed", snapshot);
    for label in touched {
        sync_title(app, snapshot, label);
    }
}

/// A window's title follows its ACTIVE tab; the main window keeps its
/// product name.
fn sync_title(app: &tauri::AppHandle, snapshot: &Snapshot, label: &str) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    if label == "main" {
        let _ = window.set_title("ObjectiveAI Viewer");
        return;
    }
    let title = snapshot
        .windows
        .get(label)
        .and_then(|wt| wt.tabs.iter().find(|t| t.id == wt.active))
        .map(|t| t.title.clone())
        .unwrap_or_else(|| "ObjectiveAI Viewer".to_string());
    let _ = window.set_title(&title);
}

/// The registry snapshot — the shell's boot read. (Subscribe to
/// `tabs://changed` FIRST, then snapshot; apply either only when the
/// generation advances.)
#[tauri::command]
pub async fn tabs_snapshot(
    registry: tauri::State<'_, TabRegistry>,
) -> Result<Snapshot, String> {
    Ok(registry.snapshot())
}

/// Open `kind`: if a tab with this exact kind exists ANYWHERE,
/// activate + focus it (open-or-focus, like the old bespoke windows);
/// otherwise append a fresh tab to the CALLING window and activate
/// it.
#[tauri::command]
pub async fn tabs_open(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, TabRegistry>,
    kind: TabKind,
) -> Result<(), String> {
    let caller = window.label().to_string();
    let (snapshot, focus, touched) = {
        let mut inner = registry.inner.lock().unwrap();
        if let Some((label, tab_id)) = inner.locate_kind(&kind) {
            // Dedupe hit — activate there. TOCTOU on the window itself
            // is handled below (a vanished window falls back to
            // appending on the caller).
            if app.get_webview_window(&label).is_some() {
                if let Some(wt) = inner.windows.get_mut(&label) {
                    wt.active = tab_id;
                }
                inner.generation += 1;
                (inner.snapshot(), Some(label.clone()), vec![label])
            } else {
                // The registry said yes but the window is gone (a
                // Destroyed cleanup is in flight) — drop the stale
                // entry and append on the caller.
                inner.windows.remove(&label);
                let tab = inner.mint_tab(kind);
                let id = tab.id;
                let wt = inner.windows.entry(caller.clone()).or_default();
                wt.tabs.push(tab);
                wt.active = id;
                inner.generation += 1;
                (inner.snapshot(), None, vec![caller])
            }
        } else {
            let tab = inner.mint_tab(kind);
            let id = tab.id;
            let wt = inner.windows.entry(caller.clone()).or_default();
            wt.tabs.push(tab);
            wt.active = id;
            inner.generation += 1;
            (inner.snapshot(), None, vec![caller])
        }
    };
    publish(&app, &snapshot, &touched);
    if let Some(label) = focus {
        if let Some(target) = app.get_webview_window(&label) {
            let _ = target.set_focus();
        }
    }
    Ok(())
}

/// Activate a tab in the calling window. Unknown ids no-op.
#[tauri::command]
pub async fn tabs_select(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, TabRegistry>,
    tab_id: u64,
) -> Result<(), String> {
    let caller = window.label().to_string();
    let snapshot = {
        let mut inner = registry.inner.lock().unwrap();
        let Some(wt) = inner.windows.get_mut(&caller) else {
            return Ok(());
        };
        if !wt.tabs.iter().any(|t| t.id == tab_id) || wt.active == tab_id {
            return Ok(());
        }
        wt.active = tab_id;
        inner.generation += 1;
        inner.snapshot()
    };
    publish(&app, &snapshot, &[caller]);
    Ok(())
}

/// Close a tab (idempotent). The neighbor (previous index, else
/// first) becomes active. A SHELL window whose last tab closes is
/// itself closed; the main window never auto-closes (it renders an
/// empty state instead).
#[tauri::command]
pub async fn tabs_close(
    app: tauri::AppHandle,
    registry: tauri::State<'_, TabRegistry>,
    tab_id: u64,
) -> Result<(), String> {
    let (snapshot, close_window, touched) = {
        let mut inner = registry.inner.lock().unwrap();
        let Some((label, idx)) = inner.locate(tab_id) else {
            return Ok(());
        };
        let wt = inner.windows.get_mut(&label).expect("located above");
        if !wt.tabs[idx].closable {
            return Ok(());
        }
        wt.tabs.remove(idx);
        if wt.active == tab_id {
            wt.active = wt
                .tabs
                .get(idx.saturating_sub(1))
                .or_else(|| wt.tabs.first())
                .map(|t| t.id)
                .unwrap_or(0);
        }
        let close_window = wt.tabs.is_empty() && label != "main";
        // The Destroyed cleanup removes the registry entry; leaving it
        // until then keeps the close idempotent.
        inner.generation += 1;
        (inner.snapshot(), close_window.then(|| label.clone()), vec![label])
    };
    publish(&app, &snapshot, &touched);
    if let Some(label) = close_window {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }
    Ok(())
}

/// Reorder a tab WITHIN the calling window (cross-window moves ride
/// the dock flow, not this). `index` is clamped.
#[tauri::command]
pub async fn tabs_move(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, TabRegistry>,
    tab_id: u64,
    index: usize,
) -> Result<(), String> {
    let caller = window.label().to_string();
    let snapshot = {
        let mut inner = registry.inner.lock().unwrap();
        let Some(wt) = inner.windows.get_mut(&caller) else {
            return Ok(());
        };
        let Some(from) = wt.tabs.iter().position(|t| t.id == tab_id) else {
            return Ok(());
        };
        let tab = wt.tabs.remove(from);
        let to = index.min(wt.tabs.len());
        wt.tabs.insert(to, tab);
        inner.generation += 1;
        inner.snapshot()
    };
    publish(&app, &snapshot, &[caller]);
    Ok(())
}

/// Tear `tab_id` out of the calling window into a FRESH shell window
/// under the cursor, then hand the user's still-held drag to the OS
/// (`start_dragging`). Idempotent per tab (a second racing call finds
/// the tab already moved and no-ops). A 1-tab shell window skips the
/// pointless rebuild: the whole window IS the tab — just start
/// dragging it.
#[tauri::command]
pub async fn tabs_detach(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    registry: tauri::State<'_, TabRegistry>,
    tab_id: u64,
) -> Result<(), String> {
    let caller = window.label().to_string();

    // 1-tab shell: drag the source window itself.
    {
        let inner = registry.inner.lock().unwrap();
        let is_sole = caller != "main"
            && inner
                .windows
                .get(&caller)
                .is_some_and(|wt| wt.tabs.len() == 1 && wt.tabs[0].id == tab_id);
        if is_sole {
            drop(inner);
            let _ = window.start_dragging();
            return Ok(());
        }
    }

    // Placement BEFORE the (slow) window build; re-anchored again just
    // before start_dragging, since the cursor keeps moving meanwhile.
    let cursor = app.cursor_position().ok();

    // Move the tab into a freshly minted shell entry FIRST — the new
    // window's boot snapshot must find its tabs.
    let (label, snapshot, touched) = {
        let mut inner = registry.inner.lock().unwrap();
        let Some((holder, idx)) = inner.locate(tab_id) else {
            // Already moved by a racing detach — idempotent no-op.
            return Ok(());
        };
        if holder != caller {
            return Ok(());
        }
        inner.next_shell += 1;
        let label = format!("shell-{}", inner.next_shell);
        let wt = inner.windows.get_mut(&caller).expect("located above");
        let tab = wt.tabs.remove(idx);
        if wt.active == tab.id {
            wt.active = wt
                .tabs
                .get(idx.saturating_sub(1))
                .or_else(|| wt.tabs.first())
                .map(|t| t.id)
                .unwrap_or(0);
        }
        let id = tab.id;
        inner
            .windows
            .insert(label.clone(), WindowTabs { tabs: vec![tab], active: id });
        inner.generation += 1;
        (label, inner.snapshot(), vec![caller.clone()])
    };
    publish(&app, &snapshot, &touched);

    let mut builder = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title(
        snapshot
            .windows
            .get(&label)
            .and_then(|wt| wt.tabs.first())
            .map(|t| t.title.as_str())
            .unwrap_or("ObjectiveAI Viewer"),
    )
    .inner_size(1024.0, 768.0);
    if let Some(cursor) = &cursor {
        // Grab point ≈ inside the new window's strip, a bit in from
        // the corner. Physical cursor → logical position via the
        // SOURCE window's scale (best available guess pre-build).
        let scale = window.scale_factor().unwrap_or(1.0);
        builder = builder.position(
            cursor.x / scale - 60.0,
            cursor.y / scale - 20.0,
        );
    }
    let new_window = match builder.build() {
        Ok(w) => w,
        Err(e) => {
            // Roll back: the tab returns to the source window, the
            // orphan entry dies — a registry entry with no window is
            // unreachable forever (Destroyed never fires for a window
            // that never existed).
            let snapshot = {
                let mut inner = registry.inner.lock().unwrap();
                if let Some(moved) = inner.windows.remove(&label) {
                    let wt = inner.windows.entry(caller.clone()).or_default();
                    wt.tabs.extend(moved.tabs);
                    if moved.active != 0 {
                        wt.active = moved.active;
                    }
                }
                inner.generation += 1;
                inner.snapshot()
            };
            publish(&app, &snapshot, &[caller]);
            return Err(format!("detach: window build failed: {e}"));
        }
    };

    // Re-anchor at the CURRENT cursor (it moved during the build),
    // then hand the still-held drag to the OS. Both best-effort: a
    // released button just leaves the window placed where it is, and
    // tao's ReleaseCapture can Err benignly.
    if let Ok(cursor) = app.cursor_position() {
        let scale = new_window.scale_factor().unwrap_or(1.0);
        let _ = new_window.set_position(tauri::LogicalPosition::new(
            cursor.x / scale - 60.0,
            cursor.y / scale - 20.0,
        ));
    }
    let _ = new_window.start_dragging();
    Ok(())
}
