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

/// What a tab IS — one UNIFORM shape for every identity: the
/// component coordinates the generic bootstrap dereferences
/// (`import(module)[export]`), plus opaque props. Built-in tabs are
/// identity `objectiveai`; plugins will be their own identity, with
/// `module` resolved against that identity's root. Rust stores all
/// of it verbatim — there is no name→component table anywhere.
/// Kind equality (identity + module + export + arguments; title is
/// cosmetic and lives outside) is the open-or-focus dedupe key.
/// The TS mirror lives in the frontend's `lib/tabs.ts`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TabKind {
    /// The OWNING identity — always derived from the opener's
    /// webview, never claimed by arguments.
    pub identity: String,
    /// Component module path, relative to the identity's root.
    pub module: String,
    /// The export holding the component (`None` = `"default"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export: Option<String>,
    /// Opaque component props — stored verbatim, delivered verbatim
    /// at boot. Rust never looks inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// The ROOT identity — what a CHROME webview speaks as. This is the
/// ONLY identity fact Rust knows; it knows no tab names and no
/// module paths for anyone, the root included (the chrome seeds the
/// home tabs through the same `tabs_open` API a plugin will use).
pub const ROOT_IDENTITY: &str = "objectiveai";

#[derive(Debug, Clone, serde::Serialize)]
pub struct Tab {
    pub id: u64,
    pub kind: TabKind,
    pub title: String,
    pub closable: bool,
    /// Optional identity icon, identity-root-relative — cosmetic,
    /// like `title` (not part of the dedupe kind).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
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
    fn mint_tab(
        &mut self,
        kind: TabKind,
        title: String,
        closable: bool,
        icon: Option<String>,
    ) -> Tab {
        self.next_tab += 1;
        Tab {
            id: self.next_tab,
            title,
            closable,
            icon,
            kind,
        }
    }
}

/// What [`ShellModel::open_or_focus`] did.
pub struct Opened {
    pub snapshot: Snapshot,
    /// The opened tab's id — fresh on an append, the EXISTING tab's
    /// on a dedupe hit.
    pub tab_id: u64,
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
    /// The new window's initial title (`<identity> - <tab name>`).
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

/// The shared removal tail: drop the tab at `(label, idx)`, activate
/// the neighbor (previous index, else first), report an emptied
/// window. Both `close` (closable-gated) and `remove_by_kind`
/// (toggle-driven, ungated) land here.
fn remove_at(inner: &mut Inner, label: String, idx: usize) -> Closed {
    let ws = inner.windows.get_mut(&label).expect("caller located");
    let tab_id = ws.tabs[idx].id;
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
    Closed {
        snapshot: inner.snapshot(),
        close_window: close_window.then(|| label.clone()),
        touched: vec![label],
    }
}

impl ShellModel {
    /// A model holding one EMPTY boot window. Rust seeds no tabs —
    /// it knows none: the boot chrome opens the home tabs through
    /// `tabs_open`, the same API every identity uses. Returns
    /// `(model, the boot window's label)`.
    pub fn boot() -> (Self, String) {
        let mut inner = Inner::default();
        inner.next_shell += 1;
        let label = format!("shell-{}", inner.next_shell);
        inner
            .windows
            .insert(label.clone(), WindowState::default());
        inner.generation += 1;
        (
            Self {
                inner: tokio::sync::Mutex::new(inner),
            },
            label,
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
    /// window's stale entry is dropped), it wins the dedupe;
    /// otherwise append a fresh tab (opener-titled, opener-closable)
    /// to `caller`. `activate` decides whether the opened/found tab
    /// becomes its window's active one (user-driven opens: yes; a
    /// toggle-on quietly appends: no). A dedupe hit keeps the
    /// EXISTING tab's title/closable — those are cosmetic, not
    /// identity.
    pub async fn open_or_focus(
        &self,
        caller: &str,
        kind: TabKind,
        title: String,
        closable: bool,
        icon: Option<String>,
        activate: bool,
        window_alive: impl Fn(&str) -> bool,
    ) -> Opened {
        let mut inner = self.inner.lock().await;
        if let Some((label, tab_id)) = inner.locate_kind(&kind) {
            if window_alive(&label) {
                if activate {
                    if let Some(ws) = inner.windows.get_mut(&label) {
                        ws.active = tab_id;
                    }
                }
                inner.generation += 1;
                return Opened {
                    snapshot: inner.snapshot(),
                    tab_id,
                    focus: activate.then(|| label.clone()),
                    touched: vec![label],
                };
            }
            // The model said yes but the window is gone (a Destroyed
            // cleanup is in flight) — drop the stale entry and append
            // on the caller.
            inner.windows.remove(&label);
        }
        let tab = inner.mint_tab(kind, title, closable, icon);
        let id = tab.id;
        let ws = inner.windows.entry(caller.to_string()).or_default();
        ws.tabs.push(tab);
        if activate || ws.active == 0 {
            // A quiet append still adopts activation in a previously
            // EMPTY window — a window must not sit with no active tab.
            ws.active = id;
        }
        inner.generation += 1;
        Opened {
            snapshot: inner.snapshot(),
            tab_id: id,
            focus: None,
            touched: vec![caller.to_string()],
        }
    }

    /// One tab, whole, by id (`None` = no such tab) — the boot-time
    /// `tab_self` read.
    pub async fn tab(&self, tab_id: u64) -> Option<Tab> {
        let inner = self.inner.lock().await;
        let (label, idx) = inner.locate(tab_id)?;
        Some(inner.windows[&label].tabs[idx].clone())
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
    /// (keeps the close idempotent). Refuses non-closable tabs — the
    /// toggle path uses [`remove_by_kind`](Self::remove_by_kind)
    /// instead.
    pub async fn close(&self, tab_id: u64) -> Option<Closed> {
        let mut inner = self.inner.lock().await;
        let (label, idx) = inner.locate(tab_id)?;
        if !inner.windows[&label].tabs[idx].closable {
            return None;
        }
        Some(remove_at(&mut inner, label, idx))
    }

    /// Remove the tab matching `kind`, wherever it lives, closable or
    /// NOT — the tabs-tab toggle's close path (home tabs are
    /// non-closable by strip rules, but a toggle may retire them).
    /// Same neighbor-activation + empty-window semantics as `close`.
    pub async fn remove_by_kind(&self, kind: &TabKind) -> Option<Closed> {
        let mut inner = self.inner.lock().await;
        let (label, tab_id) = inner.locate_kind(kind)?;
        let idx = inner.windows[&label]
            .tabs
            .iter()
            .position(|t| t.id == tab_id)?;
        Some(remove_at(&mut inner, label, idx))
    }

    /// Remove ONE tab owned by `identity` (the versioned plugin
    /// identity, `owner/name/v1.2.3`), wherever it lives, closable or
    /// not — the uninstall flow's drain (`while let`). Kind matching
    /// won't do here: channel-handler tabs carry per-offer
    /// `arguments`, so only the identity is common ground. Same
    /// neighbor-activation + empty-window semantics as `close`.
    pub async fn remove_by_identity(&self, identity: &str) -> Option<Closed> {
        let mut inner = self.inner.lock().await;
        let (label, idx) = inner.windows.iter().find_map(|(label, ws)| {
            ws.tabs
                .iter()
                .position(|t| t.kind.identity == identity)
                .map(|idx| (label.clone(), idx))
        })?;
        Some(remove_at(&mut inner, label, idx))
    }

    /// The live tab id for `kind`, if one exists.
    pub async fn tab_id_of(&self, kind: &TabKind) -> Option<u64> {
        let inner = self.inner.lock().await;
        inner.locate_kind(kind).map(|(_, id)| id)
    }

    /// Reorder every window's INVENTORY tabs to `ordered_kinds` — a
    /// SLOT permutation per window: the indices currently holding
    /// matching tabs are refilled in the given order, kinds not
    /// present are skipped (a strip-closed plugin tab), and every
    /// non-matching (dynamic) tab keeps its exact index. `active` is
    /// an id — untouched. `None` = nothing actually moved.
    pub async fn reorder_all(&self, ordered_kinds: &[TabKind]) -> Option<Snapshot> {
        let mut inner = self.inner.lock().await;
        let mut changed = false;
        for ws in inner.windows.values_mut() {
            let slots: Vec<usize> = ws
                .tabs
                .iter()
                .enumerate()
                .filter(|(_, t)| ordered_kinds.contains(&t.kind))
                .map(|(i, _)| i)
                .collect();
            if slots.len() < 2 {
                continue;
            }
            let mut desired: Vec<Tab> = Vec::with_capacity(slots.len());
            for kind in ordered_kinds {
                if let Some(pos) = ws.tabs.iter().position(|t| &t.kind == kind) {
                    desired.push(ws.tabs[pos].clone());
                }
            }
            let before: Vec<u64> = slots.iter().map(|&i| ws.tabs[i].id).collect();
            for (&slot, tab) in slots.iter().zip(desired.into_iter()) {
                ws.tabs[slot] = tab;
            }
            let after: Vec<u64> = slots.iter().map(|&i| ws.tabs[i].id).collect();
            if before != after {
                changed = true;
            }
        }
        if !changed {
            return None;
        }
        inner.generation += 1;
        Some(inner.snapshot())
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
        let title = format!("{} - {}", tab.kind.identity, tab.title);
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
