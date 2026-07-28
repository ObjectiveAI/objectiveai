//! The development-plugin registry: which plugin coordinates build
//! from a local directory instead of their git tag.
//!
//! IN-PROCESS and nothing else — no table, no file. A registration is
//! a developer's session state, and one that survived a daemon restart
//! would be a stale override silently outliving the work it was for.
//! Losing them on restart is the feature, not a limitation.
//!
//! It lives on [`crate::context::ResidentHubs`], so only the resident
//! daemon has one; every command that touches it says so rather than
//! pretending to succeed in a process that cannot serve completions
//! anyway.

use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;

/// The lookup key: canonical `(owner, name, version)`.
///
/// Canonicalization has to match what an AGENT's declaration goes
/// through, or a registration would simply never be found:
/// `agent::plugin::prepare` lowercases owner and name and leaves the
/// version verbatim (it is a git tag, and those are case-sensitive).
/// [`key`] is the single place that is applied.
pub type PluginKey = (String, String, String);

/// The cheap-clone registry handle held on
/// [`crate::context::ResidentHubs`] — one half per manifest half,
/// registered independently, exactly as the manifest splits them.
///
/// `mcp` registrations reroute the laboratory's plugin-image build to
/// a local directory; `viewer` registrations reroute the viewer's
/// `plugin://` serving to one, pushed to it over its stdin.
#[derive(Clone, Default)]
pub struct DevelopmentPlugins {
    pub mcp: HalfRegistry,
    pub viewer: HalfRegistry,
    /// The viewer APP itself, run from source — a singleton, not a
    /// per-plugin registration. See [`ViewerApp`].
    pub viewer_app: ViewerApp,
}

impl DevelopmentPlugins {
    pub fn new() -> Self {
        Self::default()
    }
}

/// One half's registrations: canonical trio → source directory.
#[derive(Clone, Default)]
pub struct HalfRegistry {
    plugins: Arc<DashMap<PluginKey, PathBuf>>,
}

/// Canonicalize a trio for lookup or storage. Owner and name
/// lowercased, version untouched.
pub fn key(owner: &str, name: &str, version: &str) -> PluginKey {
    (
        owner.trim().to_ascii_lowercase(),
        name.trim().to_ascii_lowercase(),
        version.trim().to_string(),
    )
}

impl HalfRegistry {
    /// Register a directory, returning the path it displaced.
    ///
    /// Replacing is allowed and reported rather than refused: pointing
    /// a plugin at a different checkout mid-session is ordinary, and
    /// making the caller delete first would only add a step. Whatever
    /// was produced from the displaced directory self-corrects — the
    /// mcp half's image carries its source directory as a label, and
    /// the viewer half serves live with nothing cached.
    pub fn insert(&self, key: PluginKey, path: PathBuf) -> Option<PathBuf> {
        self.plugins.insert(key, path)
    }

    /// The directory registered for these coordinates, if any.
    pub fn get(&self, key: &PluginKey) -> Option<PathBuf> {
        self.plugins.get(key).map(|entry| entry.value().clone())
    }

    /// Drop a registration, returning the path it held.
    pub fn remove(&self, key: &PluginKey) -> Option<PathBuf> {
        self.plugins.remove(key).map(|(_, path)| path)
    }

    /// Every registration, sorted by coordinates so repeated `list`
    /// calls read the same way — a `DashMap`'s own iteration order is
    /// arbitrary and would shuffle between runs.
    pub fn list(&self) -> Vec<(PluginKey, PathBuf)> {
        let mut all: Vec<(PluginKey, PathBuf)> = self
            .plugins
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all
    }
}

/// The viewer-app development slot: when set, `viewer spawn` runs
/// `pnpm exec tauri dev` in this directory instead of the installed
/// binary. A SINGLETON — there is one viewer — and in-memory like the
/// plugin registrations: a source-checkout override must not survive
/// the daemon that was told about it.
#[derive(Clone, Default)]
pub struct ViewerApp {
    path: Arc<std::sync::RwLock<Option<PathBuf>>>,
}

impl ViewerApp {
    /// Register a source directory, returning the one it displaced.
    pub fn set(&self, path: PathBuf) -> Option<PathBuf> {
        self.path
            .write()
            .expect("viewer app slot poisoned")
            .replace(path)
    }

    /// The registered source directory, if any.
    pub fn get(&self) -> Option<PathBuf> {
        self.path.read().expect("viewer app slot poisoned").clone()
    }

    /// Drop the registration, returning what it held.
    pub fn clear(&self) -> Option<PathBuf> {
        self.path.write().expect("viewer app slot poisoned").take()
    }
}
