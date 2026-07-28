//! Development-registered viewer plugins: live-served, watched,
//! hot-reloaded.
//!
//! The registry mirrors the daemon's: canonical `(owner, name,
//! version)` → the plugin's source directory. It arrives over stdin
//! (`SetDevelopmentPlugins`, the full desired state each time) and is
//! EMPTY in every viewer the daemon has nothing to tell — which is
//! what keeps the dev-aware code paths in `protocol`/`plugins`/
//! `browser` unconditional: an empty registry is byte-for-byte
//! today's behavior.
//!
//! Alongside the registry live the two attribution maps that make
//! watching PER-COMPONENT rather than per-plugin:
//!
//! - `consumed`: dev-served file → the tabs that fetched it, recorded
//!   by the `plugin://` handler (which knows the requesting webview).
//!   The set a tab actually fetched — entry, chunks, styles — is the
//!   truth; the manifest only names entries.
//! - `scripts`: browser tab → the script file injected into it,
//!   recorded at spawn. A changed script cannot be hot-swapped (an
//!   executed IIFE cannot be unspliced), so its consumers are CLOSED —
//!   honest teardown over stale code.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Canonical trio: owner and name lowercased, version verbatim — the
/// same canonicalization as the daemon registry, or a registration
/// would never be found.
pub type DevKey = (String, String, String);

pub fn dev_key(owner: &str, name: &str, version: &str) -> DevKey {
    (
        owner.trim().to_ascii_lowercase(),
        name.trim().to_ascii_lowercase(),
        version.trim().to_string(),
    )
}

#[derive(Default)]
pub struct DevPlugins {
    plugins: RwLock<HashMap<DevKey, PathBuf>>,
    consumed: RwLock<HashMap<PathBuf, HashSet<u64>>>,
    scripts: RwLock<HashMap<u64, PathBuf>>,
}

// Half these methods have callers only in `stdio`-gated modules;
// a featureless build still compiles them (the registry is managed
// unconditionally) and must not warn for it.
#[cfg_attr(not(feature = "stdio"), allow(dead_code))]
impl DevPlugins {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the registry wholesale (the wire is declarative),
    /// returning every trio whose registration CHANGED — added,
    /// removed, or re-pointed — so the caller can reload exactly those
    /// plugins' open tabs. Attribution recorded under a changed trio's
    /// old root is pruned; it will re-record on the next fetch.
    pub fn set(
        &self,
        plugins: impl IntoIterator<Item = (DevKey, PathBuf)>,
    ) -> Vec<DevKey> {
        let new: HashMap<DevKey, PathBuf> = plugins.into_iter().collect();
        let mut changed: Vec<DevKey> = Vec::new();
        let mut stale_roots: Vec<PathBuf> = Vec::new();
        {
            let mut current = self.plugins.write().expect("dev registry poisoned");
            for (key, path) in &*current {
                if new.get(key) != Some(path) {
                    changed.push(key.clone());
                    stale_roots.push(path.clone());
                }
            }
            for key in new.keys() {
                if !current.contains_key(key) {
                    changed.push(key.clone());
                }
            }
            *current = new;
        }
        if !stale_roots.is_empty() {
            let mut consumed =
                self.consumed.write().expect("dev attribution poisoned");
            consumed
                .retain(|file, _| !stale_roots.iter().any(|root| file.starts_with(root)));
        }
        changed.sort();
        changed.dedup();
        changed
    }

    /// The registered source directory for these coordinates, if any.
    pub fn get(&self, owner: &str, name: &str, version: &str) -> Option<PathBuf> {
        self.plugins
            .read()
            .expect("dev registry poisoned")
            .get(&dev_key(owner, name, version))
            .cloned()
    }

    /// Every registration — the watcher's arm list.
    pub fn roots(&self) -> Vec<(DevKey, PathBuf)> {
        let mut all: Vec<(DevKey, PathBuf)> = self
            .plugins
            .read()
            .expect("dev registry poisoned")
            .iter()
            .map(|(key, path)| (key.clone(), path.clone()))
            .collect();
        all.sort();
        all
    }

    /// Record that `tab` fetched `file` (dev-served files only — the
    /// protocol handler is the sole caller).
    pub fn record_consumed(&self, file: PathBuf, tab: u64) {
        self.consumed
            .write()
            .expect("dev attribution poisoned")
            .entry(file)
            .or_default()
            .insert(tab);
    }

    /// The tabs that fetched `file`.
    pub fn tabs_consuming(&self, file: &Path) -> Vec<u64> {
        self.consumed
            .read()
            .expect("dev attribution poisoned")
            .get(file)
            .map(|tabs| tabs.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Record the script FILE injected into browser `tab` at spawn.
    pub fn record_script(&self, tab: u64, file: PathBuf) {
        self.scripts
            .write()
            .expect("dev scripts poisoned")
            .insert(tab, file);
    }

    /// The browser tabs whose injected script came from `file`.
    pub fn browsers_running_script(&self, file: &Path) -> Vec<u64> {
        self.scripts
            .read()
            .expect("dev scripts poisoned")
            .iter()
            .filter(|(_, path)| path.as_path() == file)
            .map(|(tab, _)| *tab)
            .collect()
    }

    /// Forget a closed tab in both attribution maps. Stale entries are
    /// harmless (events to a dead webview no-op) — this just keeps the
    /// maps from growing for the viewer's whole life.
    pub fn drop_tab(&self, tab: u64) {
        self.scripts.write().expect("dev scripts poisoned").remove(&tab);
        let mut consumed = self.consumed.write().expect("dev attribution poisoned");
        for tabs in consumed.values_mut() {
            tabs.remove(&tab);
        }
        consumed.retain(|_, tabs| !tabs.is_empty());
    }
}

/// The dev-mode ASSET ROOT for a registered plugin: the directory the
/// author's watch build writes, resolved from the LIVE manifest —
/// `<root>/<viewer.development.output>`. `None` when the manifest is
/// missing/invalid, has no viewer half, or declares no
/// `viewer.development` (registration alone does not invent a layout).
pub async fn dev_asset_root(root: &Path) -> Option<PathBuf> {
    let manifest = read_dev_manifest(root).await?;
    let development = manifest.viewer.as_ref()?.development.as_ref()?;
    let mut dir = root.to_path_buf();
    for segment in development.output.split('/').filter(|s| !s.is_empty()) {
        dir = dir.join(segment);
    }
    Some(dir)
}

/// The LIVE manifest at a registered plugin's root. Uncached, like
/// every other manifest read in the viewer.
pub async fn read_dev_manifest(
    root: &Path,
) -> Option<objectiveai_sdk::cli::plugins::Manifest> {
    let bytes = tokio::fs::read(root.join("objectiveai.json")).await.ok()?;
    let manifest =
        serde_json::from_slice::<objectiveai_sdk::cli::plugins::Manifest>(&bytes)
            .ok()?;
    manifest.validate().ok()?;
    Some(manifest)
}
