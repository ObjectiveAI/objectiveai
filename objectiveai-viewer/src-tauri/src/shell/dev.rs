//! Development-registered viewer plugins: live-served, watched,
//! hot-reloaded.
//!
//! The registry is the daemon's, delivered as ARGV at spawn
//! (`--development-plugin <owner>/<name>/<version>=<path>`, one per
//! registration) and IMMUTABLE for this process's life — a
//! registration change respawns the viewer, which is the entire
//! propagation mechanism. The immutability is load-bearing for
//! simplicity: no live channel, no converge, no re-arm; the plain map
//! is read lock-free by every consumer.
//!
//! A registration REPLACES the installed plugin of the same
//! (owner, name) in discovery, and the overlap is closed by GATES
//! rather than precedence rules: while a plugin is in development
//! mode, installing it and uninstalling it both error
//! (`shell::install`), so the install tree cannot change under a
//! registration and the registration cannot change without a respawn.
//!
//! EMPTY in every viewer spawned without registrations — which is
//! what keeps the dev-aware code paths in `protocol`/`plugins`/
//! `browser` unconditional: an empty registry is byte-for-byte
//! today's behavior. Only the argv parsing and the directory watcher
//! are feature-gated (`development`).
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
    /// Immutable after construction — see the module doc.
    plugins: HashMap<DevKey, PathBuf>,
    consumed: RwLock<HashMap<PathBuf, HashSet<u64>>>,
    scripts: RwLock<HashMap<u64, PathBuf>>,
}

// Some methods have callers only in `development`-gated modules; a
// featureless build still compiles them all (the registry is managed
// unconditionally) and must not warn for it.
#[cfg_attr(not(feature = "development"), allow(dead_code))]
impl DevPlugins {
    /// An empty registry — the featureless build's only constructor
    /// (a `development` build always constructs from argv, even when
    /// the argv holds no registrations).
    #[cfg(not(feature = "development"))]
    pub fn empty() -> Self {
        Self::default()
    }

    /// The registry as parsed from argv, fixed for the process's life.
    #[cfg(feature = "development")]
    pub fn new(plugins: HashMap<DevKey, PathBuf>) -> Self {
        Self {
            plugins,
            ..Self::default()
        }
    }

    /// Parse the daemon-passed argv into a registry.
    ///
    /// Tolerant on purpose: a stray launch with unrecognized arguments
    /// (or a malformed entry) degrades to an empty registry with a
    /// stderr note, never a refused GUI start. (CEF helper processes
    /// re-exec this binary with their own argv, but they bail out in
    /// `main` before any of this runs.) The daemon-composed entries
    /// are `<owner>/<name>/<version>=<path>`, split at the FIRST `=` —
    /// the trio's charset excludes `=`, a path may not.
    #[cfg(feature = "development")]
    pub fn from_argv() -> HashMap<DevKey, PathBuf> {
        use clap::Parser as _;

        #[derive(clap::Parser)]
        struct DevArgs {
            #[arg(long = "development-plugin")]
            development_plugin: Vec<String>,
        }

        let args = match DevArgs::try_parse() {
            Ok(args) => args,
            Err(e) => {
                eprintln!("viewer: argv ignored: {e}");
                return HashMap::new();
            }
        };
        let mut plugins = HashMap::new();
        for entry in args.development_plugin {
            let parsed = entry.split_once('=').and_then(|(identity, path)| {
                let segments: Vec<&str> = identity.split('/').collect();
                match segments.as_slice() {
                    [owner, name, version] => {
                        Some((dev_key(owner, name, version), PathBuf::from(path)))
                    }
                    _ => None,
                }
            });
            match parsed {
                Some((key, path)) => {
                    plugins.insert(key, path);
                }
                None => {
                    eprintln!("viewer: malformed --development-plugin {entry:?}")
                }
            }
        }
        plugins
    }

    /// The registered source directory for these coordinates, if any.
    /// Lock-free — the map never changes.
    pub fn get(&self, owner: &str, name: &str, version: &str) -> Option<PathBuf> {
        self.plugins.get(&dev_key(owner, name, version)).cloned()
    }

    /// Whether ANY version of `(owner, name)` is registered — the
    /// install/uninstall gate: a plugin in development mode may not be
    /// installed or uninstalled, which is what keeps the replacement
    /// story free of overlap windows.
    pub fn is_dev_plugin(&self, owner: &str, name: &str) -> bool {
        let owner = owner.trim().to_ascii_lowercase();
        let name = name.trim().to_ascii_lowercase();
        self.plugins
            .keys()
            .any(|(o, n, _)| *o == owner && *n == name)
    }

    /// Every registration — the watcher's arm list and the discovery
    /// overlay's source.
    pub fn roots(&self) -> Vec<(DevKey, PathBuf)> {
        let mut all: Vec<(DevKey, PathBuf)> = self
            .plugins
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
