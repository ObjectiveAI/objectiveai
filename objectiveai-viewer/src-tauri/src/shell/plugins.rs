//! The viewer-plugin LOADER: at boot, walk the installed-plugin tree
//! (`<OBJECTIVEAI_DIR>/bin/plugins/<owner>/<name>/<version>/`) with
//! tokio fs, read each version root's `objectiveai.json`, keep the
//! HIGHEST semver per (owner, name), and open every regular tab the
//! winning manifest declares — through the SAME internal open path
//! `tabs_open` uses. Rust hardcodes no names here: identity comes
//! from the PATH (owner and name LOWERCASED — the manifest never
//! states its own identity), and tabs/modules/icons come from
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
    /// The parsed version (ordering only).
    pub version: semver::Version,
    /// The exact `<version>` path segment — the v-prefixed git tag,
    /// byte-for-byte (`v1.2.3`). THE identity segment: it matches the
    /// wire's plugin `version` verbatim, so offer lookups and tab
    /// identities line up with the install path.
    pub version_tag: String,
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
                // Version dirs are the v-prefixed git tag verbatim
                // (`v1.2.3` — the same rule agent plugin declarations
                // enforce); the semver body after the `v` orders them.
                let Some(version) = version_raw
                    .strip_prefix('v')
                    .and_then(|body| semver::Version::parse(body).ok())
                else {
                    super::report_shell(
                        app,
                        "warn",
                        format!(
                            "plugins: {owner}/{name}: skipping version dir {version_raw:?} (expected a v-prefixed semver tag, e.g. v1.2.3)"
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
                                    "plugins: {owner}/{name}@{version_raw}: invalid objectiveai.json: {e}"
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
                    version_tag: version_raw,
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

/// One installed plugin VERSION — the plugins tab's list row. Unlike
/// [`scan`], the list keeps EVERY version (uninstall targets exact
/// versions, so the surface must show them all).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersionInfo {
    /// Lowercased `<owner>` path segment.
    pub owner: String,
    /// Lowercased `<name>` path segment.
    pub name: String,
    /// The exact `<version>` dir name — the v-prefixed git tag
    /// (`v1.2.3`), byte-for-byte.
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the manifest declares a viewer extension.
    pub has_viewer: bool,
}

/// Walk owner → name → version keeping EVERY version with a parseable
/// manifest (same skip rules as [`scan`]: non-v-tag dirs warn,
/// manifest-less dirs are silently not ours). Sorted by (owner, name)
/// then version DESCENDING — newest first within a plugin.
pub(crate) async fn list_all_versions(
    app: &tauri::AppHandle,
    plugins_root: &Path,
) -> Vec<PluginVersionInfo> {
    let mut out: Vec<(semver::Version, PluginVersionInfo)> = Vec::new();
    for (owner_raw, owner_dir) in subdirs(plugins_root).await {
        let owner = owner_raw.to_lowercase();
        for (name_raw, name_dir) in subdirs(&owner_dir).await {
            let name = name_raw.to_lowercase();
            for (version_raw, version_dir) in subdirs(&name_dir).await {
                let Some(version) = version_raw
                    .strip_prefix('v')
                    .and_then(|body| semver::Version::parse(body).ok())
                else {
                    super::report_shell(
                        app,
                        "warn",
                        format!(
                            "plugins: {owner}/{name}: skipping version dir {version_raw:?} (expected a v-prefixed semver tag, e.g. v1.2.3)"
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
                                    "plugins: {owner}/{name}@{version_raw}: invalid objectiveai.json: {e}"
                                ),
                            )
                            .await;
                            continue;
                        }
                    },
                    Err(_) => continue,
                };
                let info = PluginVersionInfo {
                    owner: owner.clone(),
                    name: name.clone(),
                    version: version_raw,
                    description: manifest.description.clone(),
                    has_viewer: manifest.viewer.is_some(),
                };
                out.push((version, info));
            }
        }
    }
    out.sort_by(|(va, a), (vb, b)| {
        (&a.owner, &a.name)
            .cmp(&(&b.owner, &b.name))
            .then_with(|| vb.cmp(va))
    });
    out.into_iter().map(|(_, info)| info).collect()
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
pub(crate) async fn read_manifest(
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
    // No declared viewer root = no viewer extension at all.
    manifest.viewer.as_ref()?;
    normalize(manifest.icon?.as_str())
}

/// One resolved channel handler, ready to open as a tab (titled by
/// its offer key — handlers carry no title of their own).
pub(crate) struct PluginChannel {
    /// Root-relative module path (root = the plugin's viewer dir).
    pub module: String,
    pub export: Option<String>,
    /// The plugin's manifest icon, normalized.
    pub icon: Option<String>,
}

/// Where an offer's `(plugin, key)` pair stands against the installed
/// tree — the request tab's whole vocabulary.
pub(crate) enum ChannelStatus {
    /// That EXACT version is not installed (or its manifest is
    /// unreadable — indistinguishable and treated the same).
    NotInstalled,
    /// Installed, but the key maps to nothing: no viewer extension,
    /// no `Channel` tab with that `channel_key`, or an invalid
    /// handler module path.
    UnsupportedKey,
    /// Installed and handled — ready to accept.
    Ready(PluginChannel),
}

/// Resolve `key` against one exact installed version: the FIRST
/// `viewer.tabs`
/// [`Channel`](objectiveai_sdk::cli::plugins::ViewerTab::Channel)
/// entry whose `channel_key` matches (manifest order; later
/// duplicates are ignored).
pub(crate) async fn channel_status(
    plugins_root: &Path,
    owner: &str,
    name: &str,
    version: &str,
    key: &str,
) -> ChannelStatus {
    use objectiveai_sdk::cli::plugins::ViewerTab;
    let Some(manifest) = read_manifest(plugins_root, owner, name, version).await
    else {
        return ChannelStatus::NotInstalled;
    };
    // No declared viewer root = no viewer extension at all.
    if manifest.viewer.is_none() {
        return ChannelStatus::UnsupportedKey;
    }
    let icon = manifest.icon.as_deref().and_then(normalize);
    let handler = manifest.tabs.iter().flatten().find_map(|tab| match tab {
        ViewerTab::Channel {
            channel_key,
            module,
            export,
        } if channel_key == key => Some((module.clone(), export.clone())),
        _ => None,
    });
    let Some((module, export)) = handler else {
        return ChannelStatus::UnsupportedKey;
    };
    match normalize(&module) {
        Some(module) => ChannelStatus::Ready(PluginChannel {
            module,
            export,
            icon,
        }),
        None => ChannelStatus::UnsupportedKey,
    }
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
        // No declared viewer root = no viewer extension at all.
        if plugin.manifest.viewer.is_none() {
            continue;
        }
        // Display identity INCLUDES the version — multiple versions
        // can be installed, and the surface must say which one is
        // running. Slash-joined, mirroring the install path itself
        // (bin/plugins/<owner>/<name>/<version>), version_tag
        // verbatim so it equals the wire trio's slash-join (channel
        // handler tabs share it). The PERSISTENCE key is version-LESS
        // so toggle state survives upgrades.
        let identity =
            format!("{}/{}/{}", plugin.owner, plugin.name, plugin.version_tag);
        let identity_key = format!("{}/{}", plugin.owner, plugin.name);
        let icon = match plugin.manifest.icon.as_deref().map(normalize) {
            Some(None) => {
                super::report_shell(
                    app,
                    "warn",
                    format!(
                        "plugins: {identity}: invalid icon path {:?}",
                        plugin.manifest.icon
                    ),
                )
                .await;
                None
            }
            Some(icon) => icon,
            None => None,
        };
        let Some(tabs) = &plugin.manifest.tabs else {
            continue;
        };
        // Channel-handler entries never open at boot and never join
        // the inventory — the accept flow resolves them on demand
        // (`plugin_channel`, which dedups by channel_key alone: the
        // first entry for a key wins, and handlers freely share
        // modules/exports). Regular tabs dedup on (module, export) —
        // the live shell collapses identical kinds at open anyway
        // (title is cosmetic, not identity), so extra declarations
        // would only be pane rows for tabs that can never exist. The
        // FIRST declaration wins, title included.
        let mut seen = std::collections::HashSet::new();
        for tab in tabs {
            let objectiveai_sdk::cli::plugins::ViewerTab::Tab {
                title,
                module,
                export,
            } = tab
            else {
                continue;
            };
            let Some(module) = normalize(module) else {
                super::report_shell(
                    app,
                    "warn",
                    format!("plugins: {identity}: invalid tab module path {module:?}"),
                )
                .await;
                continue;
            };
            if !seen.insert((module.clone(), export.clone())) {
                continue;
            }
            // The tab's stable NAME (the version-less persistence key,
            // with identity_key) is its normalized module path plus,
            // for a non-default export, a `#export` suffix — the
            // manifest carries no separate name.
            let name = match export {
                Some(export) => format!("{module}#{export}"),
                None => module.clone(),
            };
            out.push(super::TabEntry {
                identity: identity.clone(),
                identity_key: identity_key.clone(),
                name,
                title: title.clone(),
                module,
                export: export.clone(),
                icon: icon.clone(),
                closable: true,
                permanent: false,
            });
        }
    }
    out
}
