//! The TAB INVENTORY: every tab loaded from the system THIS boot —
//! the root tabs the CHROME declares (`tabs_declare`; Rust hardcodes
//! no names) plus every scanned plugin tab — with a persisted
//! enabled/disabled toggle per tab. The tabs tab lists this; the
//! boot orchestrator opens the enabled slice of it.
//!
//! Persistence lives beside the log sinks:
//! `<dir>/state/<state>/viewer/tabs.json` —
//! `{ "<identity_key>": { "<tab name>": bool } }`, OVERRIDES only
//! (missing = enabled; enabling deletes the key). The whole map is
//! loaded at boot and written back whole on every toggle, so entries
//! for identities NOT loaded this boot survive untouched — a
//! disabled plugin that is uninstalled and later reinstalled comes
//! back disabled. `identity_key` is VERSION-LESS (`owner/name`; the
//! root is `objectiveai`) so toggle state survives plugin upgrades.
//!
//! PERMANENT entries (the tabs tab itself) ignore overrides
//! entirely: always open at boot, always reported enabled — which
//! also guarantees the pane's own window survives every toggle and
//! boot never yields a tab-less window.

use std::collections::HashMap;
use std::path::PathBuf;

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
    pub closable: bool,
    /// Not toggleable, always open, always enabled (the tabs tab).
    pub permanent: bool,
}

impl TabEntry {
    /// The kind this entry opens as — also the toggle-off lookup key.
    pub fn kind(&self) -> TabKind {
        TabKind {
            identity: self.identity.clone(),
            module: self.module.clone(),
            export: self.export.clone(),
            arguments: None,
        }
    }
}

/// A [`TabEntry`] plus its resolved toggle — the pane's wire shape.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryEntry {
    #[serde(flatten)]
    pub entry: TabEntry,
    pub enabled: bool,
}

/// `identity_key → tab name → enabled` — the persisted overrides.
type Overrides = HashMap<String, HashMap<String, bool>>;

/// The managed inventory.
pub struct TabInventory {
    inner: tokio::sync::Mutex<InventoryInner>,
    /// Taken by the FIRST chrome declaration (later declarations
    /// find `None` and no-op); the paired receiver gates the boot
    /// orchestrator.
    declared_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    declared_rx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
}

struct InventoryInner {
    path: PathBuf,
    roots: Vec<TabEntry>,
    plugins: Vec<TabEntry>,
    overrides: Overrides,
}

impl TabInventory {
    pub fn new(path: PathBuf) -> Self {
        let (tx, rx) = tokio::sync::oneshot::channel();
        Self {
            inner: tokio::sync::Mutex::new(InventoryInner {
                path,
                roots: Vec::new(),
                plugins: Vec::new(),
                overrides: HashMap::new(),
            }),
            declared_tx: tokio::sync::Mutex::new(Some(tx)),
            declared_rx: tokio::sync::Mutex::new(Some(rx)),
        }
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

    /// Load the persisted overrides (missing file = empty, silent;
    /// corrupt file = warn + empty — the next toggle overwrites).
    pub async fn load_overrides(&self, app: &tauri::AppHandle) {
        let path = self.inner.lock().await.path.clone();
        let bytes = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(_) => return,
        };
        match serde_json::from_slice::<Overrides>(&bytes) {
            Ok(overrides) => self.inner.lock().await.overrides = overrides,
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

    /// The full inventory, toggle-resolved: roots in declaration
    /// order, then plugins in scan order. Missing override = enabled;
    /// permanent = ALWAYS enabled.
    pub async fn inventory(&self) -> Vec<InventoryEntry> {
        let inner = self.inner.lock().await;
        inner
            .roots
            .iter()
            .chain(inner.plugins.iter())
            .map(|entry| InventoryEntry {
                enabled: entry.permanent
                    || inner
                        .overrides
                        .get(&entry.identity_key)
                        .and_then(|tabs| tabs.get(&entry.name))
                        .copied()
                        .unwrap_or(true),
                entry: entry.clone(),
            })
            .collect()
    }

    /// One entry by its toggle coordinates.
    pub async fn entry(&self, identity_key: &str, name: &str) -> Option<TabEntry> {
        let inner = self.inner.lock().await;
        inner
            .roots
            .iter()
            .chain(inner.plugins.iter())
            .find(|e| e.identity_key == identity_key && e.name == name)
            .cloned()
    }

    /// Persist one toggle: enabling DELETES the override (missing =
    /// enabled keeps the file minimal), disabling stores `false`.
    /// The whole map is written back — entries for identities not
    /// loaded this boot are preserved.
    pub async fn set_override(
        &self,
        identity_key: &str,
        name: &str,
        enabled: bool,
    ) -> std::io::Result<()> {
        let (path, overrides) = {
            let mut inner = self.inner.lock().await;
            if enabled {
                if let Some(tabs) = inner.overrides.get_mut(identity_key) {
                    tabs.remove(name);
                    if tabs.is_empty() {
                        inner.overrides.remove(identity_key);
                    }
                }
            } else {
                inner
                    .overrides
                    .entry(identity_key.to_string())
                    .or_default()
                    .insert(name.to_string(), false);
            }
            (inner.path.clone(), inner.overrides.clone())
        };
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        let json = serde_json::to_vec_pretty(&overrides)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        tokio::fs::write(&path, json).await
    }
}

/// The BOOT orchestrator: await the chrome's declaration and the
/// plugin scan together, load the persisted toggles, then open every
/// ENABLED entry — roots in declaration order, plugins after — into
/// the boot window, and hand focus back to the first of them. This
/// is the ONLY boot-time opener; with nothing installed and nothing
/// disabled it reproduces the old seeding exactly.
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
        inventory.load_overrides(&app).await;

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
