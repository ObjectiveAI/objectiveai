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
/// [`crate::context::ResidentHubs`].
#[derive(Clone, Default)]
pub struct DevelopmentPlugins {
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

impl DevelopmentPlugins {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a directory, returning the path it displaced.
    ///
    /// Replacing is allowed and reported rather than refused: pointing
    /// a plugin at a different checkout mid-session is ordinary, and
    /// making the caller delete first would only add a step. The
    /// displaced directory's image stays tagged until something
    /// rebuilds — which the next create does on its own, since the
    /// image carries its source directory as a label.
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
