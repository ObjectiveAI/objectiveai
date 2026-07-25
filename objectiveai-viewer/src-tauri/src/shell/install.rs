//! The viewer-extension INSTALLER: land a plugin's DAEMON-BUILT
//! viewer bundle under
//! `<OBJECTIVEAI_DIR>/bin/plugins/<owner>/<name>/<version>/`.
//!
//! The DAEMON is the build machine: `plugins_install` downloads the
//! finished artifact from `GET /plugins/{owner}/{name}/{version}/viewer`
//! (the daemon fetches the tag — local-override-first on ITS machine
//! — and runs the pnpm+esbuild pipeline), un-tars the streamed tar.gz
//! into this side's OWN temp partition (`<bin>/temp/viewer`), sanity-
//! checks the manifest, and renames into place. Viewer machines need
//! NO git, NO Node, NO pnpm. Lock discipline is unchanged: machine-
//! wide `plugin-viewer-*` lock, double-checked dest probe, temp
//! cleanup on every path, nothing lands unless the whole download
//! validated.
//!
//! The commands here (`plugins_list` / `plugins_install` /
//! `plugins_uninstall`) are the plugins tab's surface — root-identity
//! callers only.

use std::path::{Path, PathBuf};

use futures::StreamExt;

/// The installer's directory layout, derived from `OBJECTIVEAI_DIR`
/// once — managed state so every command shares one source of truth.
pub struct PluginsDirs {
    objectiveai_dir: PathBuf,
}

impl PluginsDirs {
    pub fn new(objectiveai_dir: PathBuf) -> Self {
        Self { objectiveai_dir }
    }

    /// `<dir>/bin/plugins` — the installed-plugin tree (machine-wide,
    /// shared across states).
    pub fn plugins_root(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join("plugins")
    }

    /// `<dir>/bin/temp/viewer` — the viewer's OWN temp partition
    /// (download staging; the laboratory host owns the sibling
    /// `daemon` partition, the daemon's build service `daemon-viewer`).
    pub fn temp_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join("temp").join("viewer")
    }

    /// `<dir>/bin/locks` — the machine-wide lock dir the laboratory
    /// host also uses (distinct keys: `plugin-viewer-*` here,
    /// `plugin-image-*` there).
    fn locks_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join("locks")
    }
}

/// Path-meaningful characters are rejected outright in wire-supplied
/// identity segments — the same rules as the manifest readers'.
fn safe_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains('/')
        && !segment.contains('\\')
        && segment != "."
        && segment != ".."
}

/// Download the daemon-built bundle and un-tar it into `staging` —
/// the whole artifact acquisition. The daemon builds on demand
/// (fetching the tag local-override-first on ITS machine, pnpm +
/// esbuild there); this side streams tar.gz and unpacks. A truncated
/// download fails un-gzip or the manifest sanity-parse — nothing
/// lands.
async fn download_bundle(
    app: &tauri::AppHandle,
    identity: &str,
    daemon: &objectiveai_sdk::daemon::Client,
    owner: &str,
    name: &str,
    version: &str,
    staging: &Path,
) -> Result<(), String> {
    let plugin = daemon
        .get_viewer_plugin(owner, name, version)
        .await
        .map_err(|e| format!("download bundle: {e}"))?;
    if let Some(sha) = &plugin.commit_sha {
        super::report_shell(
            app,
            "info",
            format!("plugins: {identity}: bundle streaming (commit {sha})"),
        )
        .await;
    }
    tokio::fs::create_dir_all(staging)
        .await
        .map_err(|e| format!("create staging dir: {e}"))?;
    // Sync tar+gzip over the async byte stream: StreamReader bridges
    // the chunks, SyncIoBridge hands them to the blocking unpacker —
    // the mirror of the daemon's send side. Memory stays flat.
    let byte_stream = plugin
        .bytes_stream()
        .map(|chunk| chunk.map_err(std::io::Error::other));
    let reader = tokio_util::io::StreamReader::new(byte_stream);
    let staging_owned = staging.to_path_buf();
    let unpacked = tokio::task::spawn_blocking(move || {
        let bridge = tokio_util::io::SyncIoBridge::new(reader);
        let decoder = flate2::read::GzDecoder::new(bridge);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(&staging_owned)
            .map_err(|e| format!("unpack bundle: {e}"))
    })
    .await
    .map_err(|e| format!("unpack task panicked: {e}"))?;
    unpacked?;
    // Sanity: the artifact root must hold a parseable manifest — the
    // completeness check a raw byte stream can't give.
    let bytes = tokio::fs::read(staging.join("objectiveai.json"))
        .await
        .map_err(|e| format!("bundle missing objectiveai.json: {e}"))?;
    serde_json::from_slice::<objectiveai_sdk::cli::plugins::Manifest>(&bytes)
        .map_err(|e| format!("bundle manifest invalid: {e}"))?;
    Ok(())
}

/// The full install pipeline. Lock discipline mirrors the laboratory
/// host's `plugin_image::ensure`: probe → machine-wide lock →
/// re-probe → work → temp cleanup on success AND failure → explicit
/// release on EVERY path (a `LockClaim` drop deliberately does not
/// release).
pub(crate) async fn install(
    app: &tauri::AppHandle,
    daemon: &objectiveai_sdk::daemon::Client,
    dirs: &PluginsDirs,
    owner: &str,
    name: &str,
    version: &str,
) -> Result<(), String> {
    if !safe_segment(owner) || !safe_segment(name) || !safe_segment(version) {
        return Err("invalid owner/name/version".to_string());
    }
    let owner = owner.to_lowercase();
    let name = name.to_lowercase();
    // The version IS the git tag, byte-for-byte, Go-modules style —
    // the same rule the SDK enforces on agent plugin declarations
    // (`agent::plugin::Plugin::validate`). Nothing rewrites it.
    if !version.starts_with('v') {
        return Err(format!(
            "`version` {version:?} must start with 'v' — it is the plugin repo's git tag, Go-modules style (v1.2.3)",
        ));
    }
    semver::Version::parse(&version[1..])
        .map_err(|e| format!("invalid version {version:?}: {e}"))?;
    let identity = format!("{owner}/{name}@{version}");
    let dest = dirs.plugins_root().join(&owner).join(&name).join(version);
    if tokio::fs::try_exists(&dest).await.unwrap_or(false) {
        return Err(format!("{identity} is already installed"));
    }
    super::report_shell(
        app,
        "info",
        format!("plugins: {identity}: downloading the daemon-built bundle for tag {version}"),
    )
    .await;
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &dirs.locks_dir(),
        &format!("plugin-viewer-{owner}-{name}-{version}"),
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| format!("bin lock: {e}"))?;
    let result = async {
        // Double-checked: a sibling process may have installed while
        // we were blocked on the lock.
        if tokio::fs::try_exists(&dest).await.unwrap_or(false) {
            return Err(format!("{identity} is already installed"));
        }
        let staging = dirs.temp_dir().join(uuid::Uuid::new_v4().to_string());
        let landed = async {
            download_bundle(app, &identity, daemon, &owner, &name, version, &staging)
                .await?;
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("create install dir: {e}"))?;
            }
            tokio::fs::rename(&staging, &dest)
                .await
                .map_err(|e| format!("land {identity}: {e}"))
        }
        .await;
        if landed.is_err() {
            objectiveai_sdk::gitrepo::remove_checkout(&staging).await;
        }
        landed
    }
    .await;
    claim
        .release()
        .map_err(|e| format!("bin lock release: {e}"))?;
    result
}

/// Uninstall ONE exact installed version: unload every live tab
/// carrying its versioned identity FIRST (boot tabs and channel
/// handlers share it), then delete the version dir under the same
/// machine-wide lock install takes, then prune the name and owner
/// dirs bottom-up if emptied — only the path parts specific to this
/// plugin ever go.
pub(crate) async fn uninstall(
    app: &tauri::AppHandle,
    model: &super::ShellModel,
    dirs: &PluginsDirs,
    owner: &str,
    name: &str,
    version: &str,
) -> Result<(), String> {
    if !safe_segment(owner) || !safe_segment(name) || !safe_segment(version) {
        return Err("invalid owner/name/version".to_string());
    }
    let owner = owner.to_lowercase();
    let name = name.to_lowercase();
    let identity = format!("{owner}/{name}@{version}");
    let name_dir = dirs.plugins_root().join(&owner).join(&name);
    let dest = name_dir.join(version);
    if !tokio::fs::try_exists(&dest).await.unwrap_or(false) {
        return Err(format!("{identity} is not installed"));
    }
    // Drain live tabs before anything is deleted. Identity match, not
    // kind match — a dormant (non-active) version's identity is on no
    // live tab, so uninstalling it closes nothing.
    let tab_identity = format!("{owner}/{name}/{version}");
    while let Some(closed) = model.remove_by_identity(&tab_identity).await {
        super::native::publish(app, &closed.snapshot, &closed.touched);
        super::native::sync(app).await;
        if let Some(label) = closed.close_window {
            use tauri::Manager;
            if let Some(window) = app.get_window(&label) {
                let _ = window.close();
            }
        }
    }
    // The SAME lock key as install — serializes a concurrent
    // reinstall of the exact version being deleted.
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &dirs.locks_dir(),
        &format!("plugin-viewer-{owner}-{name}-{version}"),
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| format!("bin lock: {e}"))?;
    let result = async {
        if !tokio::fs::try_exists(&dest).await.unwrap_or(false) {
            return Err(format!("{identity} is not installed"));
        }
        tokio::fs::remove_dir_all(&dest)
            .await
            .map_err(|e| format!("remove {identity}: {e}"))?;
        // Bottom-up prune: `remove_dir` refuses non-empty dirs, which
        // IS the "no other names/versions" guard — best-effort by
        // design.
        if tokio::fs::remove_dir(&name_dir).await.is_ok() {
            if let Some(owner_dir) = name_dir.parent() {
                let _ = tokio::fs::remove_dir(owner_dir).await;
            }
        }
        Ok(())
    }
    .await;
    claim
        .release()
        .map_err(|e| format!("bin lock release: {e}"))?;
    result
}

/// Install a plugin's viewer extension — the plugins tab's Install
/// button. Runs the whole pipeline inline (the JS awaits), then
/// rescans the inventory and quietly opens the new enabled tabs at
/// their config-order slots.
#[tauri::command]
pub async fn plugins_install(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, super::ShellModel>,
    dirs: tauri::State<'_, PluginsDirs>,
    owner: String,
    name: String,
    version: String,
) -> Result<(), String> {
    if super::sender_identity(&webview, &model).await != super::ROOT_IDENTITY {
        return Err("plugins_install: root identity only".to_string());
    }
    let window = webview.window().label().to_string();
    let proxy = {
        use tauri::Manager;
        app.state::<crate::daemon_proxy::DaemonProxy>()
    };
    match install(&app, proxy.daemon(), &dirs, &owner, &name, &version).await {
        Ok(()) => {
            super::report_shell(
                &app,
                "info",
                format!("plugins: {owner}/{name}@{version}: installed"),
            )
            .await;
            super::rescan_and_apply(&app, &dirs.plugins_root(), &window, true).await;
            Ok(())
        }
        Err(e) => {
            super::report_shell(
                &app,
                "error",
                format!("plugins: install {owner}/{name}@{version}: {e}"),
            )
            .await;
            Err(e)
        }
    }
}

/// Uninstall a plugin version — the plugins tab's per-row Uninstall
/// button. Unloads its tabs, deletes it, then rescans: if an older
/// installed version resurfaces as the active one, its enabled tabs
/// open at their config-order slots (what the next boot would show).
#[tauri::command]
pub async fn plugins_uninstall(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, super::ShellModel>,
    dirs: tauri::State<'_, PluginsDirs>,
    owner: String,
    name: String,
    version: String,
) -> Result<(), String> {
    if super::sender_identity(&webview, &model).await != super::ROOT_IDENTITY {
        return Err("plugins_uninstall: root identity only".to_string());
    }
    let window = webview.window().label().to_string();
    match uninstall(&app, &model, &dirs, &owner, &name, &version).await {
        Ok(()) => {
            super::report_shell(
                &app,
                "info",
                format!("plugins: {owner}/{name}@{version}: uninstalled"),
            )
            .await;
            super::rescan_and_apply(&app, &dirs.plugins_root(), &window, true).await;
            Ok(())
        }
        Err(e) => {
            super::report_shell(
                &app,
                "error",
                format!("plugins: uninstall {owner}/{name}@{version}: {e}"),
            )
            .await;
            Err(e)
        }
    }
}

/// Every installed plugin VERSION, for the plugins tab's list — the
/// full tree, not `scan`'s highest-per-plugin reduction.
#[tauri::command]
pub async fn plugins_list(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, super::ShellModel>,
    dirs: tauri::State<'_, PluginsDirs>,
) -> Result<Vec<super::PluginVersionInfo>, String> {
    if super::sender_identity(&webview, &model).await != super::ROOT_IDENTITY {
        return Err("plugins_list: root identity only".to_string());
    }
    Ok(super::list_all_versions(&app, &dirs.plugins_root()).await)
}
