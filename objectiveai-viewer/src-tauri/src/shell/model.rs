//! The shell MODEL — the single source of truth for which tabs live
//! in which window, which is active, and each window's chrome-driven
//! UI state (zoom/orientation). Pure state: mutation methods return
//! what changed and NEVER touch windows, webviews, or the event bus —
//! [`super::native`] turns snapshots into native effects and
//! [`super::commands`] orchestrates the two.
//!
//! Every mutation bumps a `generation`; consumers (the chrome
//! webviews) apply a snapshot — event or boot read — only when its
//! generation advances, so a stale snapshot can never clobber a newer
//! event. UI state is deliberately NOT in the snapshot: it rides
//! targeted `ui://changed` events to the content webviews (a
//! zoom-slider drag must not broadcast full snapshots).
//!
//! The lock is a `tokio::sync::Mutex` (project rule: never
//! `std::sync`); guards are still dropped before emits/native ops out
//! of discipline, but holding across an await is safe where needed.

use std::collections::HashMap;

/// What a tab IS — everything needed to render its content anywhere.
/// Serde-tagged; the TS mirror lives in the frontend's `lib/tabs.ts`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TabKind {
    /// The agents home (watermark + hierarchy tree).
    Agents,
    /// The laboratories home (laboratory builder / list).
    Laboratories,
    /// The viewer's own log inbox — everything the capture
    /// initialization script hoovers out of every webview (console,
    /// uncaught errors, unhandled rejections; see `shell/logs.rs`).
    ViewerLogs,
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
            TabKind::ViewerLogs => "viewer logs".to_string(),
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

    /// Whether the shell shows a close button. The home tabs are
    /// permanent (movable, never closable).
    fn closable(&self) -> bool {
        !matches!(
            self,
            TabKind::Agents | TabKind::Laboratories | TabKind::ViewerLogs
        )
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

/// One window's chrome-driven UI state. Owned by the window (a
/// content webview adopts whichever window it currently sits in —
/// only Rust reliably knows that after a reparent).
#[derive(Debug, Clone, serde::Serialize)]
pub struct UiState {
    /// Canvas zoom factor (1 = 100%).
    pub zoom: f64,
    /// Hierarchy descent direction: `"vertical"` | `"horizontal"`.
    pub orientation: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            orientation: "vertical".to_string(),
        }
    }
}

/// One window's full model state. [`WindowTabs`] is its serialized
/// projection (tabs + active only — UI state rides targeted events).
#[derive(Debug, Clone, Default)]
pub struct WindowState {
    pub tabs: Vec<Tab>,
    /// The active tab's id (0 = none, only possible on an empty
    /// window).
    pub active: u64,
    pub ui: UiState,
}

/// One window's slice of the snapshot.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct WindowTabs {
    pub tabs: Vec<Tab>,
    pub active: u64,
}

/// The whole model, as snapshotted to the chrome webviews.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Snapshot {
    pub generation: u64,
    pub windows: HashMap<String, WindowTabs>,
}

#[derive(Debug, Default)]
struct Inner {
    windows: HashMap<String, WindowState>,
    generation: u64,
    next_tab: u64,
    next_shell: u64,
}

impl Inner {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            generation: self.generation,
            windows: self
                .windows
                .iter()
                .map(|(label, ws)| {
                    (
                        label.clone(),
                        WindowTabs {
                            tabs: ws.tabs.clone(),
                            active: ws.active,
                        },
                    )
                })
                .collect(),
        }
    }

    /// Find a tab by id: `(window label, index)`.
    fn locate(&self, tab_id: u64) -> Option<(String, usize)> {
        for (label, ws) in &self.windows {
            if let Some(idx) = ws.tabs.iter().position(|t| t.id == tab_id) {
                return Some((label.clone(), idx));
            }
        }
        None
    }

    /// Find a window already holding `kind`: `(label, tab id)`.
    fn locate_kind(&self, kind: &TabKind) -> Option<(String, u64)> {
        for (label, ws) in &self.windows {
            if let Some(tab) = ws.tabs.iter().find(|t| &t.kind == kind) {
                return Some((label.clone(), tab.id));
            }
        }
        None
    }

    /// Tab ids are monotonic and NEVER reused — content-webview
    /// labels (`tab-<id>`) can therefore never collide across a tab's
    /// whole close/reopen life.
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

/// What [`ShellModel::open_or_focus`] did.
pub struct Opened {
    pub snapshot: Snapshot,
    /// `Some(label)` = dedupe hit: activate + focus that window.
    pub focus: Option<String>,
    /// Windows whose titles may have changed.
    pub touched: Vec<String>,
}

/// What [`ShellModel::close`] did.
pub struct Closed {
    pub snapshot: Snapshot,
    /// A SHELL window left empty by the close — the caller closes it.
    pub close_window: Option<String>,
    pub touched: Vec<String>,
}

/// What [`ShellModel::detach_to`] did.
pub struct Detached {
    /// The freshly minted shell window label the tab moved to.
    pub label: String,
    /// The moved tab's title (the new window's initial title).
    pub title: String,
    pub snapshot: Snapshot,
    pub touched: Vec<String>,
}

/// The managed model. Seeded at construction (setup-time), mutated
/// only through the methods below (commands + docking + the Destroyed
/// cleanup).
pub struct ShellModel {
    inner: tokio::sync::Mutex<Inner>,
}

impl ShellModel {
    /// A model pre-seeded with the boot window's initial tab set (the
    /// generation starts advanced so the first snapshot is post-seed).
    /// The boot window is NOT special — it's minted like any other
    /// shell; returns `(model, its label, its initial title)`.
    pub fn seeded(kinds: Vec<TabKind>) -> (Self, String, String) {
        let mut inner = Inner::default();
        inner.next_shell += 1;
        let label = format!("shell-{}", inner.next_shell);
        let tabs: Vec<Tab> = kinds.into_iter().map(|k| inner.mint_tab(k)).collect();
        let active = tabs.first().map(|t| t.id).unwrap_or(0);
        let title = tabs
            .first()
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "ObjectiveAI Viewer".to_string());
        inner.windows.insert(
            label.clone(),
            WindowState {
                tabs,
                active,
                ui: UiState::default(),
            },
        );
        inner.generation += 1;
        (
            Self {
                inner: tokio::sync::Mutex::new(inner),
            },
            label,
            title,
        )
    }

    pub async fn snapshot(&self) -> Snapshot {
        self.inner.lock().await.snapshot()
    }

    /// Every window's FULL state (tabs + active + ui) — the
    /// reconciler's view.
    pub async fn windows_full(&self) -> HashMap<String, WindowState> {
        self.inner.lock().await.windows.clone()
    }

    /// Open `kind`: if a tab with this exact kind exists ANYWHERE (and
    /// its window is still alive per `window_alive` — a vanished
    /// window's stale entry is dropped), activate it there; otherwise
    /// append a fresh tab to `caller` and activate it.
    pub async fn open_or_focus(
        &self,
        caller: &str,
        kind: TabKind,
        window_alive: impl Fn(&str) -> bool,
    ) -> Opened {
        let mut inner = self.inner.lock().await;
        if let Some((label, tab_id)) = inner.locate_kind(&kind) {
            if window_alive(&label) {
                if let Some(ws) = inner.windows.get_mut(&label) {
                    ws.active = tab_id;
                }
                inner.generation += 1;
                return Opened {
                    snapshot: inner.snapshot(),
                    focus: Some(label.clone()),
                    touched: vec![label],
                };
            }
            // The model said yes but the window is gone (a Destroyed
            // cleanup is in flight) — drop the stale entry and append
            // on the caller.
            inner.windows.remove(&label);
        }
        let tab = inner.mint_tab(kind);
        let id = tab.id;
        let ws = inner.windows.entry(caller.to_string()).or_default();
        ws.tabs.push(tab);
        ws.active = id;
        inner.generation += 1;
        Opened {
            snapshot: inner.snapshot(),
            focus: None,
            touched: vec![caller.to_string()],
        }
    }

    /// Activate a tab in `caller`. Unknown ids / no-change no-op.
    pub async fn select(&self, caller: &str, tab_id: u64) -> Option<Snapshot> {
        let mut inner = self.inner.lock().await;
        let ws = inner.windows.get_mut(caller)?;
        if !ws.tabs.iter().any(|t| t.id == tab_id) || ws.active == tab_id {
            return None;
        }
        ws.active = tab_id;
        inner.generation += 1;
        Some(inner.snapshot())
    }

    /// Close a tab (idempotent). The neighbor (previous index, else
    /// first) becomes active. Reports a window left empty — EVERY
    /// window closes when its last tab does (no window is special);
    /// its model entry stays until the window's Destroyed cleanup
    /// (keeps the close idempotent).
    pub async fn close(&self, tab_id: u64) -> Option<Closed> {
        let mut inner = self.inner.lock().await;
        let (label, idx) = inner.locate(tab_id)?;
        let ws = inner.windows.get_mut(&label).expect("located above");
        if !ws.tabs[idx].closable {
            return None;
        }
        ws.tabs.remove(idx);
        if ws.active == tab_id {
            ws.active = ws
                .tabs
                .get(idx.saturating_sub(1))
                .or_else(|| ws.tabs.first())
                .map(|t| t.id)
                .unwrap_or(0);
        }
        let close_window = ws.tabs.is_empty();
        inner.generation += 1;
        Some(Closed {
            snapshot: inner.snapshot(),
            close_window: close_window.then(|| label.clone()),
            touched: vec![label],
        })
    }

    /// Reorder a tab WITHIN `caller` (cross-window moves ride the dock
    /// flow, not this). `index` is clamped.
    pub async fn move_tab(&self, caller: &str, tab_id: u64, index: usize) -> Option<Snapshot> {
        let mut inner = self.inner.lock().await;
        let ws = inner.windows.get_mut(caller)?;
        let from = ws.tabs.iter().position(|t| t.id == tab_id)?;
        let tab = ws.tabs.remove(from);
        let to = index.min(ws.tabs.len());
        ws.tabs.insert(to, tab);
        inner.generation += 1;
        Some(inner.snapshot())
    }

    /// Is `tab_id` the SOLE tab of `label`? (The detach shortcut: the
    /// whole window IS the tab — just drag the window.)
    pub async fn is_sole_tab(&self, label: &str, tab_id: u64) -> bool {
        let inner = self.inner.lock().await;
        inner
            .windows
            .get(label)
            .is_some_and(|ws| ws.tabs.len() == 1 && ws.tabs[0].id == tab_id)
    }

    /// Move `tab_id` out of `caller` into a freshly minted shell
    /// entry (UI state starts at defaults — a new window). `None` =
    /// already moved by a racing detach, or not the caller's tab.
    pub async fn detach_to(&self, caller: &str, tab_id: u64) -> Option<Detached> {
        let mut inner = self.inner.lock().await;
        let (holder, idx) = inner.locate(tab_id)?;
        if holder != caller {
            return None;
        }
        inner.next_shell += 1;
        let label = format!("shell-{}", inner.next_shell);
        let ws = inner.windows.get_mut(caller).expect("located above");
        let tab = ws.tabs.remove(idx);
        if ws.active == tab.id {
            ws.active = ws
                .tabs
                .get(idx.saturating_sub(1))
                .or_else(|| ws.tabs.first())
                .map(|t| t.id)
                .unwrap_or(0);
        }
        let id = tab.id;
        let title = tab.title.clone();
        inner.windows.insert(
            label.clone(),
            WindowState {
                tabs: vec![tab],
                active: id,
                ui: UiState::default(),
            },
        );
        inner.generation += 1;
        Some(Detached {
            label: label.clone(),
            title,
            snapshot: inner.snapshot(),
            touched: vec![caller.to_string()],
        })
    }

    /// Undo a [`detach_to`](Self::detach_to) whose window build
    /// failed: the tab returns to `caller`, the orphan entry dies (an
    /// entry with no window is unreachable forever — Destroyed never
    /// fires for a window that never existed).
    pub async fn rollback_detach(&self, label: &str, caller: &str) -> Snapshot {
        let mut inner = self.inner.lock().await;
        if let Some(moved) = inner.windows.remove(label) {
            let ws = inner.windows.entry(caller.to_string()).or_default();
            ws.tabs.extend(moved.tabs);
            if moved.active != 0 {
                ws.active = moved.active;
            }
        }
        inner.generation += 1;
        inner.snapshot()
    }

    /// Move every tab of `source` onto the end of `target` (the dock
    /// merge). The moved window's active tab becomes the target's
    /// active — the dragged window's visible tab stays visible.
    pub async fn merge_windows(&self, source: &str, target: &str) -> Option<Snapshot> {
        let mut inner = self.inner.lock().await;
        if !inner.windows.contains_key(target) {
            return None;
        }
        let moved = inner.windows.remove(source)?;
        let target_ws = inner.windows.get_mut(target).expect("checked above");
        let follow_active = moved.active;
        target_ws.tabs.extend(moved.tabs);
        if follow_active != 0 {
            target_ws.active = follow_active;
        }
        inner.generation += 1;
        Some(inner.snapshot())
    }

    /// Remove a window's entry entirely (window destroyed).
    /// Idempotent; `None` = nothing changed.
    pub async fn remove_window(&self, label: &str) -> Option<Snapshot> {
        let mut inner = self.inner.lock().await;
        inner.windows.remove(label)?;
        inner.generation += 1;
        Some(inner.snapshot())
    }

    /// Merge UI fields into `window`'s state; returns the merged
    /// state + the window's tab ids (the `ui://changed` targets).
    /// `None` = unknown window (mid-destroy) — nothing to drive.
    pub async fn set_ui(
        &self,
        window: &str,
        zoom: Option<f64>,
        orientation: Option<String>,
    ) -> Option<(UiState, Vec<u64>)> {
        let mut inner = self.inner.lock().await;
        let ws = inner.windows.get_mut(window)?;
        if let Some(zoom) = zoom {
            ws.ui.zoom = zoom;
        }
        if let Some(orientation) = orientation {
            ws.ui.orientation = orientation;
        }
        Some((ws.ui.clone(), ws.tabs.iter().map(|t| t.id).collect()))
    }

    /// A tab's title, wherever it lives (`None` = no such tab).
    pub async fn tab_title(&self, tab_id: u64) -> Option<String> {
        let inner = self.inner.lock().await;
        let (label, idx) = inner.locate(tab_id)?;
        Some(inner.windows[&label].tabs[idx].title.clone())
    }

    /// `window`'s UI state (defaults for unknown windows).
    pub async fn ui_for_window(&self, window: &str) -> UiState {
        let inner = self.inner.lock().await;
        inner
            .windows
            .get(window)
            .map(|ws| ws.ui.clone())
            .unwrap_or_default()
    }
}
