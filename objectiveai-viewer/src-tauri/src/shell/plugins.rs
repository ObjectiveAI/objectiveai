//! The viewer-plugin LOADER: at boot, walk the installed-plugin tree
//! (`<OBJECTIVEAI_DIR>/bin/plugins/<owner>/<name>/<version>/`) with
//! tokio fs, read each version root's `objectiveai.json`, keep the
//! HIGHEST semver per (owner, name), and open every tab the winning
//! manifest's `viewer` section declares — through the SAME internal
//! open path `tabs_open` uses. Rust hardcodes no names here: identity
//! comes from the PATH (owner and name LOWERCASED — the manifest
//! never states its own identity), and tabs/modules/icons come from
//! manifest DATA.
//!
//! Everything is best-effort: a corrupt manifest or unreadable dir is
//! skipped with a `viewer-shell` line in the viewer-logs inbox —
//! boot must never die on a bad install. Serving the plugin's actual
//! code (the plugin:// protocol, host document, import maps) is the
//! NEXT stage; until then an opened plugin tab renders empty and its
//! failed module import lands in viewer-logs.

use std::path::{Path, PathBuf};

use objectiveai_sdk::cli::plugins::Manifest;

/// One installed plugin, at its selected (highest) version.
pub struct InstalledPlugin {
    /// Lowercased `<owner>` path segment.
    pub owner: String,
    /// Lowercased `<name>` path segment.
    pub name: String,
    pub version: semver::Version,
    pub manifest: Manifest,
}

/// The subdirectory names of `dir` (files and unreadable entries
/// skipped).
async fn subdirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return out;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if entry.file_type().await.is_ok_and(|t| t.is_dir()) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                out.push((name.to_string(), path.clone()));
            }
        }
    }
    out
}

/// Walk owner → name → version, parse manifests, select the highest
/// version per (owner, name). Identity segments are LOWERCASED (the
/// path is the identity; the manifest never states it).
pub async fn scan(app: &tauri::AppHandle, plugins_root: &Path) -> Vec<InstalledPlugin> {
    let mut selected: std::collections::HashMap<(String, String), InstalledPlugin> =
        std::collections::HashMap::new();
    for (owner_raw, owner_dir) in subdirs(plugins_root).await {
        let owner = owner_raw.to_lowercase();
        for (name_raw, name_dir) in subdirs(&owner_dir).await {
            let name = name_raw.to_lowercase();
            for (version_raw, version_dir) in subdirs(&name_dir).await {
                let Ok(version) = semver::Version::parse(&version_raw) else {
                    super::report_shell(
                        app,
                        "warn",
                        format!(
                            "plugins: {owner}/{name}: skipping non-semver version dir {version_raw:?}"
                        ),
                    )
                    .await;
                    continue;
                };
                let manifest_path = version_dir.join("objectiveai.json");
                let manifest = match tokio::fs::read(&manifest_path).await {
                    Ok(bytes) => match serde_json::from_slice::<Manifest>(&bytes) {
                        Ok(manifest) => manifest,
                        Err(e) => {
                            super::report_shell(
                                app,
                                "error",
                                format!(
                                    "plugins: {owner}/{name}@{version}: invalid objectiveai.json: {e}"
                                ),
                            )
                            .await;
                            continue;
                        }
                    },
                    // No manifest = not an installed plugin version;
                    // silently not ours to report.
                    Err(_) => continue,
                };
                let key = (owner.clone(), name.clone());
                let candidate = InstalledPlugin {
                    owner: owner.clone(),
                    name: name.clone(),
                    version,
                    manifest,
                };
                match selected.get(&key) {
                    Some(existing) if existing.version >= candidate.version => {}
                    _ => {
                        selected.insert(key, candidate);
                    }
                }
            }
        }
    }
    let mut plugins: Vec<InstalledPlugin> = selected.into_values().collect();
    plugins.sort_by(|a, b| (&a.owner, &a.name).cmp(&(&b.owner, &b.name)));
    plugins
}

/// Canonicalize a manifest path (authored relative to `viewer/`,
/// CWD-style) into the kind's uniform root-relative form: `./`
/// stripped, stored with a leading `/` whose root IS the plugin's
/// viewer dir. `None` = a path that tries to leave the root.
pub(crate) fn normalize(path: &str) -> Option<String> {
    let path = path.strip_prefix("./").unwrap_or(path);
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains("://")
        || path.split('/').any(|segment| segment == ".." || segment.is_empty())
    {
        return None;
    }
    Some(format!("/{path}"))
}

/// Read ONE exact installed version's manifest — the on-demand lookup
/// under the channel-offer flows. The trio arrives over the wire:
/// segments are rejected outright if empty or path-meaningful
/// (separators, `.`/`..`) rather than sanitized. Owner/name are
/// lowercased to match the install path (the path IS the identity);
/// the version is exact-case. `None` on any miss — best-effort,
/// silent.
async fn read_manifest(
    plugins_root: &Path,
    owner: &str,
    name: &str,
    version: &str,
) -> Option<Manifest> {
    let safe = |segment: &str| {
        !segment.is_empty()
            && !segment.contains('/')
            && !segment.contains('\\')
            && segment != "."
            && segment != ".."
    };
    if !safe(owner) || !safe(name) || !safe(version) {
        return None;
    }
    let manifest_path = plugins_root
        .join(owner.to_lowercase())
        .join(name.to_lowercase())
        .join(version)
        .join("objectiveai.json");
    let bytes = tokio::fs::read(&manifest_path).await.ok()?;
    serde_json::from_slice::<Manifest>(&bytes).ok()
}

/// The manifest icon of ONE exact installed version, normalized
/// root-relative — the channel-offer tab's icon lookup (on-demand: a
/// plugin may carry an icon while declaring no tabs, so the
/// inventory's per-tab entries can't answer this).
pub(crate) async fn plugin_icon(
    plugins_root: &Path,
    owner: &str,
    name: &str,
    version: &str,
) -> Option<String> {
    let manifest = read_manifest(plugins_root, owner, name, version).await?;
    normalize(manifest.viewer?.icon?.as_str())
}

/// One resolved channel handler, ready to open as a tab.
pub(crate) struct PluginChannel {
    /// Root-relative module path (root = the plugin's viewer dir).
    pub module: String,
    pub export: Option<String>,
    pub title: Option<String>,
    /// The plugin's manifest icon, normalized.
    pub icon: Option<String>,
}

/// The plugin's channel handler for `key` — the FIRST
/// `viewer.channels` entry whose `key` matches (manifest order).
/// `None` when the plugin isn't installed, declares no channels, has
/// no entry for the key, or the entry's module path is invalid.
pub(crate) async fn plugin_channel(
    plugins_root: &Path,
    owner: &str,
    name: &str,
    version: &str,
    key: &str,
) -> Option<PluginChannel> {
    let manifest = read_manifest(plugins_root, owner, name, version).await?;
    let viewer = manifest.viewer?;
    let icon = viewer.icon.as_deref().and_then(normalize);
    let channels = viewer.channels?;
    let entry = channels.values().find(|entry| entry.key == key)?;
    Some(PluginChannel {
        module: normalize(&entry.module)?,
        export: entry.export.clone(),
        title: entry.title.clone(),
        icon,
    })
}

/// Scan the tree and turn every declared plugin tab into a
/// [`TabEntry`](super::TabEntry) for the inventory (which owns
/// opening). With no plugins installed this collects nothing.
pub(crate) async fn collect_plugin_entries(
    app: &tauri::AppHandle,
    plugins_root: &Path,
) -> Vec<super::TabEntry> {
    let mut out = Vec::new();
    for plugin in scan(app, plugins_root).await {
        let Some(viewer) = &plugin.manifest.viewer else {
            continue;
        };
        // Display identity INCLUDES the version — multiple versions
        // can be installed, and the surface must say which one is
        // running. Slash-joined, mirroring the install path itself
        // (bin/plugins/<owner>/<name>/<version>). The PERSISTENCE
        // key is version-LESS so toggle state survives upgrades.
        let identity = format!("{}/{}/{}", plugin.owner, plugin.name, plugin.version);
        let identity_key = format!("{}/{}", plugin.owner, plugin.name);
        let icon = match viewer.icon.as_deref().map(normalize) {
            Some(None) => {
                super::report_shell(
                    app,
                    "warn",
                    format!("plugins: {identity}: invalid icon path {:?}", viewer.icon),
                )
                .await;
                None
            }
            Some(icon) => icon,
            None => None,
        };
        let Some(tabs) = &viewer.tabs else {
            continue;
        };
        for (tab_name, tab) in tabs {
            let Some(module) = normalize(&tab.module) else {
                super::report_shell(
                    app,
                    "warn",
                    format!(
                        "plugins: {identity}: tab {tab_name:?}: invalid module path {:?}",
                        tab.module
                    ),
                )
                .await;
                continue;
            };
            out.push(super::TabEntry {
                identity: identity.clone(),
                identity_key: identity_key.clone(),
                name: tab_name.clone(),
                title: tab.title.clone().unwrap_or_else(|| tab_name.clone()),
                module,
                export: tab.export.clone(),
                icon: icon.clone(),
                closable: true,
                permanent: false,
            });
        }
    }
    out
}
