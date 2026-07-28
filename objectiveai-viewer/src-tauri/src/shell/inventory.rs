//! The TAB INVENTORY: every tab loaded from the system THIS boot —
//! the root tabs the CHROME declares (`tabs_declare`; Rust hardcodes
//! no names) plus every scanned plugin tab — with a persisted
//! enabled/disabled toggle AND a persisted ORDER per tab. The tabs
//! tab lists (and reorders) this; the boot orchestrator opens the
//! enabled slice of it, in order.
//!
//! Persistence lives beside the log sinks:
//! `<dir>/state/<state>/viewer/tabs.json` — an ORDERED array
//! `[ { "identityKey", "name", "enabled" }, ... ]` whose order IS
//! the tab order (`enabled` defaults true and is omitted when true;
//! an entry may exist purely to hold a slot). The whole Vec is
//! loaded at boot and written back whole INSIDE the inventory lock
//! (serializing mutations AND their writes — two rapid reorders can
//! never leave the older Vec on disk), so entries for identities NOT
//! loaded this boot survive in position — a disabled plugin that is
//! uninstalled and later reinstalled comes back disabled, where it
//! was. `identity_key` is VERSION-LESS (`owner/name`; the root is
//! `objectiveai`) so state survives plugin upgrades.
//!
//! Every WRITE first MATERIALIZES the current display order into the
//! Vec (inserting loaded-but-unspecified entries at their displayed
//! end positions) — otherwise toggling an unspecified entry would
//! append it and visibly jump its row.
//!
//! Reorders treat UNLOADED entries as STICKY: partition the old Vec
//! into segments by the positions of loaded entries — the leading
//! unloaded segment stays start-pinned, the trailing segment stays
//! end-pinned, and each interior segment rides immediately after the
//! loaded entry that preceded it, wherever that entry moves. (The
//! tabs pane applies the same stickiness among its visible rows:
//! dragging an enabled row carries its trailing run of middle
//! disabled rows.)
//!
//! PERMANENT entries (the tabs tab itself) ignore the enabled bit
//! entirely: always open at boot, always reported enabled — which
//! also guarantees the pane's own window survives every toggle and
//! boot never yields a tab-less window.
//!
//! `user_controlled` is a SESSION flag (never persisted): once the
//! user pops a tab out or reorders the strip by hand, the live strip
//! belongs to them — pane reorders and toggle-ons still persist, but
//! stop live-applying to the strip until the next boot.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::Manager;

use super::model::TabKind;

/// One available tab, as discovered this boot.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabEntry {
    /// Display identity (versioned for plugins: `owner/name/1.2.0`).
    pub identity: String,
    /// The PERSISTENCE key: version-less (`owner/name`; root =
    /// `objectiveai`).
    pub identity_key: String,
    /// The tab's name — unique within its identity; the persistence
    /// sub-key.
    pub name: String,
    pub title: String,
    pub module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Stylesheets the tab declares, identity-root-relative —
    /// cosmetic like `icon`, and like it NOT part of the kind.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<String>,
    pub closable: bool,
    /// Not toggleable, always open, always enabled (the tabs tab).
    pub permanent: bool,
}

impl TabEntry {
    /// The kind this entry opens as — also the toggle/reorder lookup
    /// key on the live strip.
    pub fn kind(&self) -> TabKind {
        TabKind {
            identity: self.identity.clone(),
            key: None,
            arguments: None,
            surface: super::Surface::Component {
                module: self.module.clone(),
                export: self.export.clone(),
                root_module: false,
            },
        }
    }
}

/// A [`TabEntry`] plus its resolved toggle — the pane's wire shape,
/// emitted in DISPLAY ORDER.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryEntry {
    #[serde(flatten)]
    pub entry: TabEntry,
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_true(enabled: &bool) -> bool {
    *enabled
}

/// One persisted line — the file's ORDER is the tab order.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedTab {
    identity_key: String,
    name: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    enabled: bool,
}

/// The managed inventory.
pub struct TabInventory {
    inner: tokio::sync::Mutex<InventoryInner>,
    /// Taken by the FIRST chrome declaration (later declarations
    /// find `None` and no-op); the paired receiver gates the boot
    /// orchestrator.
    declared_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    declared_rx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
    /// SESSION-ONLY: the user popped a tab out or reordered the
    /// strip by hand — the live strip is theirs now.
    user_controlled: AtomicBool,
}

struct InventoryInner {
    path: PathBuf,
    roots: Vec<TabEntry>,
    plugins: Vec<TabEntry>,
    /// The persisted, ORDERED state. May hold entries for identities
    /// not loaded this boot — they keep their slots.
    order: Vec<PersistedTab>,
}

impl InventoryInner {
    fn loaded(&self) -> impl Iterator<Item = &TabEntry> {
        self.roots.iter().chain(self.plugins.iter())
    }

    fn find_loaded(&self, identity_key: &str, name: &str) -> Option<&TabEntry> {
        self.loaded()
            .find(|e| e.identity_key == identity_key && e.name == name)
    }

    /// The display order: persisted entries that are LOADED, in file
    /// order, then loaded-but-unspecified entries in discovery order.
    fn display(&self) -> Vec<InventoryEntry> {
        let mut out = Vec::new();
        let mut seen: HashSet<(&str, &str)> = HashSet::new();
        for persisted in &self.order {
            if let Some(entry) = self.find_loaded(&persisted.identity_key, &persisted.name) {
                if seen.insert((&entry.identity_key, &entry.name)) {
                    out.push(InventoryEntry {
                        enabled: entry.permanent || persisted.enabled,
                        entry: entry.clone(),
                    });
                }
            }
        }
        for entry in self.loaded() {
            if !seen.contains(&(entry.identity_key.as_str(), entry.name.as_str())) {
                out.push(InventoryEntry {
                    enabled: true,
                    entry: entry.clone(),
                });
            }
        }
        out
    }

    /// Materialize the display order into the Vec: every loaded
    /// entry gets a persisted slot (unspecified ones append at their
    /// displayed end positions, enabled). Order-identity — a
    /// prerequisite of every write, so a toggle can never move a
    /// row.
    fn materialize(&mut self) {
        let missing: Vec<(String, String)> = {
            let present: HashSet<(&str, &str)> = self
                .order
                .iter()
                .map(|p| (p.identity_key.as_str(), p.name.as_str()))
                .collect();
            self.loaded()
                .filter(|e| !present.contains(&(e.identity_key.as_str(), e.name.as_str())))
                .map(|e| (e.identity_key.clone(), e.name.clone()))
                .collect()
        };
        for (identity_key, name) in missing {
            self.order.push(PersistedTab {
                identity_key,
                name,
                enabled: true,
            });
        }
    }

    /// The STICKY reorder: `d` is the new display order of LOADED
    /// entries (all of them — enabled and disabled). Unloaded
    /// entries partition into segments by the loaded positions:
    /// leading → start-pinned, trailing → end-pinned, interior →
    /// riding after the loaded entry that preceded them.
    fn sticky_reorder(&mut self, d: &[(String, String)]) {
        let d_set: HashSet<(&str, &str)> = d
            .iter()
            .map(|(k, n)| (k.as_str(), n.as_str()))
            .collect();
        let is_member =
            |p: &PersistedTab| d_set.contains(&(p.identity_key.as_str(), p.name.as_str()));
        let last_member_pos = self.order.iter().rposition(&is_member);
        let mut start_pins: Vec<PersistedTab> = Vec::new();
        let mut end_pins: Vec<PersistedTab> = Vec::new();
        // The D-members with their enabled bits, and each member's
        // attached interior run, keyed by (identity_key, name).
        let mut members: Vec<PersistedTab> = Vec::new();
        let mut attached: std::collections::HashMap<(String, String), Vec<PersistedTab>> =
            std::collections::HashMap::new();
        let mut current: Option<(String, String)> = None;
        for (i, persisted) in self.order.drain(..).enumerate() {
            if is_member(&persisted) {
                current = Some((persisted.identity_key.clone(), persisted.name.clone()));
                members.push(persisted);
            } else if let Some(parent) = &current {
                if last_member_pos.is_some_and(|last| i > last) {
                    end_pins.push(persisted);
                } else {
                    attached.entry(parent.clone()).or_default().push(persisted);
                }
            } else {
                start_pins.push(persisted);
            }
        }
        let mut order = start_pins;
        for (identity_key, name) in d {
            let Some(pos) = members
                .iter()
                .position(|p| &p.identity_key == identity_key && &p.name == name)
            else {
                continue;
            };
            order.push(members.remove(pos));
            if let Some(run) = attached.remove(&(identity_key.clone(), name.clone())) {
                order.extend(run);
            }
        }
        order.extend(end_pins);
        self.order = order;
    }

    /// Write the whole Vec — CALLED UNDER THE LOCK by design (the
    /// lock serializes mutations and their writes together).
    async fn persist(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        let json = serde_json::to_vec_pretty(&self.order)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        tokio::fs::write(&self.path, json).await
    }
}

impl TabInventory {
    pub fn new(path: PathBuf) -> Self {
        let (tx, rx) = tokio::sync::oneshot::channel();
        Self {
            inner: tokio::sync::Mutex::new(InventoryInner {
                path,
                roots: Vec::new(),
                plugins: Vec::new(),
                order: Vec::new(),
            }),
            declared_tx: tokio::sync::Mutex::new(Some(tx)),
            declared_rx: tokio::sync::Mutex::new(Some(rx)),
            user_controlled: AtomicBool::new(false),
        }
    }

    /// The user popped a tab out or reordered the strip — the live
    /// strip is theirs for the rest of this process.
    pub fn set_user_controlled(&self) {
        self.user_controlled.store(true, Ordering::Relaxed);
    }

    pub fn user_controlled(&self) -> bool {
        self.user_controlled.load(Ordering::Relaxed)
    }

    /// Apply the chrome's root declaration — FIRST wins; later
    /// chromes (new windows re-running the same build) no-op.
    /// Returns whether this call was the one that applied.
    pub async fn declare(&self, roots: Vec<TabEntry>) -> bool {
        let Some(tx) = self.declared_tx.lock().await.take() else {
            return false;
        };
        self.inner.lock().await.roots = roots;
        let _ = tx.send(());
        true
    }

    pub async fn set_plugins(&self, plugins: Vec<TabEntry>) {
        self.inner.lock().await.plugins = plugins;
    }

    /// Load the persisted order (missing file = empty, silent;
    /// corrupt or old-format file = warn + empty — the next write
    /// overwrites). Dedupes by (identity_key, name), first wins.
    pub async fn load(&self, app: &tauri::AppHandle) {
        let path = self.inner.lock().await.path.clone();
        let bytes = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(_) => return,
        };
        match serde_json::from_slice::<Vec<PersistedTab>>(&bytes) {
            Ok(order) => {
                let mut seen: HashSet<(String, String)> = HashSet::new();
                let deduped: Vec<PersistedTab> = order
                    .into_iter()
                    .filter(|p| seen.insert((p.identity_key.clone(), p.name.clone())))
                    .collect();
                self.inner.lock().await.order = deduped;
            }
            Err(e) => {
                super::report_shell(
                    app,
                    "warn",
                    format!("tabs: invalid {}: {e} — using defaults", path.display()),
                )
                .await;
            }
        }
    }

    /// The full inventory in DISPLAY ORDER, toggle-resolved. Missing
    /// entry = enabled; permanent = ALWAYS enabled.
    pub async fn inventory(&self) -> Vec<InventoryEntry> {
        self.inner.lock().await.display()
    }

    /// The display-order KINDS (all entries, enabled and disabled) —
    /// the live-apply input.
    pub async fn display_kinds(&self) -> Vec<TabKind> {
        self.inner
            .lock()
            .await
            .display()
            .iter()
            .map(|item| item.entry.kind())
            .collect()
    }

    /// One entry by its toggle coordinates.
    pub async fn entry(&self, identity_key: &str, name: &str) -> Option<TabEntry> {
        self.inner
            .lock()
            .await
            .find_loaded(identity_key, name)
            .cloned()
    }

    /// Persist one toggle. Materializes first (a toggle must never
    /// move a row), then flips the bit in place. Write happens under
    /// the lock.
    pub async fn set_enabled(
        &self,
        identity_key: &str,
        name: &str,
        enabled: bool,
    ) -> std::io::Result<()> {
        let mut inner = self.inner.lock().await;
        inner.materialize();
        if let Some(persisted) = inner
            .order
            .iter_mut()
            .find(|p| p.identity_key == identity_key && p.name == name)
        {
            persisted.enabled = enabled;
        }
        inner.persist().await
    }

    /// Persist a new display order from the pane. `d` must be a
    /// PERMUTATION of the loaded inventory (outer `Err` = rejected,
    /// the pane re-fetches); the inner result is the disk write.
    pub async fn reorder(
        &self,
        d: Vec<(String, String)>,
    ) -> Result<std::io::Result<()>, String> {
        let mut inner = self.inner.lock().await;
        let loaded: HashSet<(String, String)> = inner
            .loaded()
            .map(|e| (e.identity_key.clone(), e.name.clone()))
            .collect();
        let proposed: HashSet<(String, String)> = d.iter().cloned().collect();
        if proposed.len() != d.len() || proposed != loaded {
            return Err("tabs_reorder: order is not a permutation of the inventory".to_string());
        }
        inner.materialize();
        inner.sticky_reorder(&d);
        Ok(inner.persist().await)
    }
}

/// The runtime plugin rescan — install and uninstall both land here:
/// refresh the inventory's plugin slice from disk, optionally open
/// every enabled-but-not-live PLUGIN entry QUIETLY into `window`
/// (the calling webview's window — `tabs_toggle`'s quiet append; the
/// restriction to plugin entries keeps a root tab the user closed
/// with its window from resurrecting), live-slot the strip to config
/// order outside user-controlled mode, and emit `inventory://changed`
/// so every open tabs pane refreshes.
pub(crate) async fn rescan_and_apply(
    app: &tauri::AppHandle,
    plugins_root: &std::path::Path,
    window: &str,
    open_missing: bool,
) {
    let inventory = app.state::<TabInventory>();
    let plugins = super::collect_plugin_entries(app, plugins_root).await;
    let plugin_keys: std::collections::HashSet<(String, String)> = plugins
        .iter()
        .map(|entry| (entry.identity_key.clone(), entry.name.clone()))
        .collect();
    inventory.set_plugins(plugins).await;
    let model = app.state::<super::ShellModel>();
    if open_missing && app.get_window(window).is_some() {
        for item in inventory.inventory().await {
            if !item.enabled
                || !plugin_keys.contains(&(
                    item.entry.identity_key.clone(),
                    item.entry.name.clone(),
                ))
            {
                continue;
            }
            let kind = item.entry.kind();
            if model.tab_id_of(&kind).await.is_some() {
                continue;
            }
            super::open_tab(
                app,
                window,
                kind,
                item.entry.title.clone(),
                item.entry.closable,
                item.entry.icon.clone(),
                item.entry.styles.clone(),
                false,
            )
            .await;
        }
    }
    if !inventory.user_controlled() {
        super::live_slot(app, &model, &inventory.display_kinds().await).await;
    }
    use tauri::Emitter;
    let _ = app.emit("inventory://changed", &inventory.inventory().await);
}

/// The BOOT orchestrator: await the chrome's declaration and the
/// plugin scan together, load the persisted order/toggles, then open
/// every ENABLED entry — in DISPLAY ORDER — into the boot window,
/// and hand focus back to the first of them. Opens are sequential
/// fast model-appends (order-deterministic) while webview LOADING
/// stays parallel (open_tab backgrounds the reconciler).
pub fn spawn_boot_orchestrator(
    app: tauri::AppHandle,
    plugins_root: PathBuf,
    boot_label: String,
) {
    tauri::async_runtime::spawn(async move {
        let inventory = app.state::<TabInventory>();
        let declared = inventory.declared_rx.lock().await.take();
        let (_, plugins) = tokio::join!(
            async {
                if let Some(rx) = declared {
                    let _ = rx.await;
                }
            },
            super::collect_plugin_entries(&app, &plugins_root)
        );
        inventory.set_plugins(plugins).await;
        inventory.load(&app).await;

        // The boot window may have died while we waited — opening
        // into a dead label would RESURRECT its model entry as an
        // unreachable ghost. Fall back to any live shell window.
        let window = if app.get_window(&boot_label).is_some() {
            boot_label
        } else {
            match app
                .windows()
                .keys()
                .find(|label| label.starts_with("shell-"))
                .cloned()
            {
                Some(label) => label,
                None => {
                    super::report_shell(
                        &app,
                        "warn",
                        "tabs: no live window at boot-open time".to_string(),
                    )
                    .await;
                    return;
                }
            }
        };

        let mut first: Option<TabKind> = None;
        for item in inventory.inventory().await {
            if !item.enabled {
                continue;
            }
            let kind = item.entry.kind();
            if first.is_none() {
                first = Some(kind.clone());
            }
            super::open_tab(
                &app,
                &window,
                kind,
                item.entry.title.clone(),
                item.entry.closable,
                item.entry.icon.clone(),
                item.entry.styles.clone(),
                true,
            )
            .await;
        }
        // Each open activated itself — hand the strip back to the
        // FIRST enabled tab.
        if let Some(kind) = first {
            let model = app.state::<super::ShellModel>();
            if let Some(tab_id) = model.tab_id_of(&kind).await {
                super::select_tab(&app, &window, tab_id).await;
            }
        }
    });
}
