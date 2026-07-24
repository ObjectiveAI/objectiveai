//! The viewer-extension INSTALLER: turn a plugin repo's declared
//! `viewer` root into installed, self-contained tab bundles under
//! `<OBJECTIVEAI_DIR>/bin/plugins/<owner>/<name>/<version>/`.
//!
//! The pipeline mirrors the laboratory host's container-image build
//! (`plugin_image::ensure` over there): machine-wide file lock →
//! double-checked probe → single-tag git fetch (local-override-first)
//! into this side's OWN temp partition (`<bin>/temp/viewer` — the
//! host owns the sibling `<bin>/temp/daemon`) → work → cleanup on
//! every path. Everything validates IN TEMP and fails loudly there;
//! nothing lands in bin/plugins unless the whole pipeline succeeds.
//!
//! The commands here (`plugins_list` / `plugins_install` /
//! `plugins_uninstall`) are the plugins tab's surface — root-identity
//! callers only.

use std::path::{Path, PathBuf};

use objectiveai_sdk::cli::plugins::{Manifest, ViewerTab};

/// Packages the HOST viewer provides at runtime (import-map-served):
/// stripped from the extension's dependency sets before install and
/// left external by the bundle.
const HOST_PACKAGES: &[&str] = &["react", "react-dom"];

/// Bare specifiers the bundle must leave external — the host serves
/// them. Superset of [`HOST_PACKAGES`]: subpath imports too.
const EXTERNALS: &[&str] =
    &["react", "react-dom", "react/jsx-runtime", "react-dom/client"];

/// The bundler, injected into the extension's devDependencies — the
/// HOST owns the bundling contract, so the author's pin (if any) is
/// overwritten.
const ESBUILD_VERSION: &str = "^0.25.0";

/// pnpm as spawned: Windows' CreateProcess won't resolve the `.cmd`
/// shim from a bare `pnpm`.
const PNPM: &str = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };

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
    /// (checkouts + staging; the laboratory host owns the sibling
    /// `daemon` partition).
    pub fn temp_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join("temp").join("viewer")
    }

    /// `<dir>/bin/locks` — the machine-wide lock dir the laboratory
    /// host also uses (distinct keys: `plugin-viewer-*` here,
    /// `plugin-image-*` there).
    fn locks_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join("locks")
    }

    /// `<dir>/plugins` — the local plugin-repo override root the
    /// fetch consults before GitHub.
    fn override_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("plugins")
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

/// Spawn a build tool windowless in `cwd` and capture its output.
/// `Ok` = full stdout. `Err` = spawn failure (NotFound gets an
/// actionable message) or a non-zero exit with the code and the last
/// ~4 KiB of combined stdout+stderr folded in.
async fn run_tool(
    program: &std::ffi::OsStr,
    args: &[&str],
    cwd: &Path,
    envs: &[(&str, &str)],
) -> Result<String, String> {
    let display = Path::new(program)
        .file_name()
        .unwrap_or(program)
        .to_string_lossy()
        .into_owned();
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    objectiveai_sdk::process::no_window(&mut cmd);
    let output = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!(
                "{display} not found — install Node.js + pnpm (a bundled pnpm is planned)"
            )
        } else {
            format!("spawn {display}: {e}")
        }
    })?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(combined)
    } else {
        let tail_start = combined
            .len()
            .saturating_sub(4096)
            .min(combined.len());
        // Don't split a UTF-8 char.
        let tail_start = (tail_start..combined.len())
            .find(|i| combined.is_char_boundary(*i))
            .unwrap_or(combined.len());
        Err(format!(
            "{display} exited with {}: {}",
            output.status,
            combined[tail_start..].trim()
        ))
    }
}

/// Rewrite the extension's `package.json` in TEMP before install:
/// strip the host-provided packages from every dependency set (STRIP,
/// not move-to-peer — pnpm's auto-install-peers default would just
/// reinstall them) and inject the host-pinned esbuild.
async fn rewrite_package_json(path: &Path) -> Result<(), String> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| format!("read package.json: {e}"))?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse package.json: {e}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "package.json is not a JSON object".to_string())?;
    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(deps) = object.get_mut(section).and_then(|v| v.as_object_mut()) {
            for package in HOST_PACKAGES {
                deps.remove(*package);
            }
        }
    }
    object
        .entry("devDependencies")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| "package.json devDependencies is not an object".to_string())?
        .insert(
            "esbuild".to_string(),
            serde_json::Value::String(ESBUILD_VERSION.to_string()),
        );
    let out = serde_json::to_vec_pretty(&value)
        .map_err(|e| format!("serialize package.json: {e}"))?;
    tokio::fs::write(path, out)
        .await
        .map_err(|e| format!("write package.json: {e}"))
}

/// Validate a repo-relative forward-slash path (the containerfile
/// rules) and resolve it under `root`.
fn resolve_repo_rel(root: &Path, path: &str, what: &str) -> Result<PathBuf, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err(format!("`{what}` cannot be empty"));
    }
    if path.contains('\\') {
        return Err(format!("`{what}` must use forward slashes"));
    }
    if path.starts_with('/') || path.contains(':') {
        return Err(format!("`{what}` must be a repo-relative path"));
    }
    if path
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(format!("`{what}` has an invalid path component: {path:?}"));
    }
    Ok(path
        .split('/')
        .fold(root.to_path_buf(), |path, component| path.join(component)))
}

/// A tab module's OUTPUT name: the source extension swapped for the
/// bundle's `.js` (esbuild's outdir naming under `--outbase=.`).
fn bundle_name(module: &str) -> String {
    for ext in [".tsx", ".ts", ".jsx", ".mjs", ".cjs", ".js"] {
        if let Some(stem) = module.strip_suffix(ext) {
            return format!("{stem}.js");
        }
    }
    format!("{module}.js")
}

/// A manifest tab's module path, whichever variant.
fn tab_module(tab: &ViewerTab) -> &str {
    match tab {
        ViewerTab::Channel { module, .. } | ViewerTab::Tab { module, .. } => module,
    }
}

/// The build half of the pipeline: manifest → validate → rewrite →
/// pnpm install → esbuild bundle → assemble the FINAL version-dir
/// layout in `staging` (rewritten manifest + `viewer/` bundles +
/// icon). Everything happens in temp; the caller lands `staging`
/// only on `Ok`.
async fn build_install(
    app: &tauri::AppHandle,
    identity: &str,
    checkout: &Path,
    staging: &Path,
) -> Result<(), String> {
    let bytes = tokio::fs::read(checkout.join("objectiveai.json"))
        .await
        .map_err(|e| format!("read objectiveai.json: {e}"))?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse objectiveai.json: {e}"))?;
    let Some(viewer_rel) = manifest.viewer.as_deref() else {
        return Err(
            "plugin declares no viewer extension (`viewer` absent from objectiveai.json)"
                .to_string(),
        );
    };
    let viewer_root = resolve_repo_rel(checkout, viewer_rel, "viewer")?;
    let package_json = viewer_root.join("package.json");
    if !tokio::fs::metadata(&package_json)
        .await
        .is_ok_and(|m| m.is_file())
    {
        return Err(format!(
            "viewer root {viewer_rel:?} has no package.json"
        ));
    }
    rewrite_package_json(&package_json).await?;
    super::report_shell(
        app,
        "info",
        format!("plugins: {identity}: installing dependencies (pnpm)"),
    )
    .await;
    run_tool(
        std::ffi::OsStr::new(PNPM),
        &["install", "--ignore-workspace", "--no-frozen-lockfile"],
        &viewer_root,
        &[("COREPACK_ENABLE_STRICT", "0")],
    )
    .await?;

    // Every declared tab module — BOTH variants (channel handlers
    // bundle too), deduped, source-verified.
    let mut seen = std::collections::HashSet::new();
    let mut entries: Vec<String> = Vec::new();
    for tab in manifest.tabs.iter().flatten() {
        let module = tab_module(tab);
        let rel = super::normalize(module)
            .ok_or_else(|| format!("invalid tab module path {module:?}"))?[1..]
            .to_string();
        if !seen.insert(rel.clone()) {
            continue;
        }
        let source = rel
            .split('/')
            .fold(viewer_root.clone(), |path, component| path.join(component));
        if !tokio::fs::metadata(&source)
            .await
            .is_ok_and(|m| m.is_file())
        {
            return Err(format!(
                "tab module {module:?} not found in the viewer root"
            ));
        }
        entries.push(rel);
    }

    let out_viewer = staging.join("viewer");
    tokio::fs::create_dir_all(&out_viewer)
        .await
        .map_err(|e| format!("create staging dir: {e}"))?;
    if !entries.is_empty() {
        super::report_shell(
            app,
            "info",
            format!(
                "plugins: {identity}: bundling {} module(s) (esbuild)",
                entries.len()
            ),
        )
        .await;
        let esbuild = viewer_root.join("node_modules").join(".bin").join(
            if cfg!(windows) { "esbuild.cmd" } else { "esbuild" },
        );
        let outdir = format!("--outdir={}", out_viewer.display());
        let mut args: Vec<&str> = entries.iter().map(String::as_str).collect();
        args.extend([
            "--bundle",
            "--format=esm",
            "--platform=browser",
            outdir.as_str(),
            "--outbase=.",
            "--jsx=automatic",
        ]);
        let externals: Vec<String> = EXTERNALS
            .iter()
            .map(|package| format!("--external:{package}"))
            .collect();
        args.extend(externals.iter().map(String::as_str));
        run_tool(esbuild.as_os_str(), &args, &viewer_root, &[]).await?;
    }

    if let Some(icon) = manifest.icon.as_deref() {
        let rel = super::normalize(icon)
            .ok_or_else(|| format!("invalid icon path {icon:?}"))?[1..]
            .to_string();
        let source = rel
            .split('/')
            .fold(viewer_root.clone(), |path, component| path.join(component));
        if !tokio::fs::metadata(&source)
            .await
            .is_ok_and(|m| m.is_file())
        {
            return Err(format!("icon {icon:?} not found in the viewer root"));
        }
        let target = rel
            .split('/')
            .fold(out_viewer.clone(), |path, component| path.join(component));
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create icon dir: {e}"))?;
        }
        tokio::fs::copy(&source, &target)
            .await
            .map_err(|e| format!("copy icon: {e}"))?;
    }

    // The INSTALLED manifest: fixed `viewer` layout, modules remapped
    // source → bundle. (Typed re-serialize — unmodeled fields drop;
    // the viewer only reads typed fields.)
    let mut installed = manifest.clone();
    installed.viewer = Some("viewer".to_string());
    if let Some(tabs) = installed.tabs.as_mut() {
        for tab in tabs {
            let module = match tab {
                ViewerTab::Channel { module, .. } | ViewerTab::Tab { module, .. } => module,
            };
            let rel = super::normalize(module)
                .ok_or_else(|| format!("invalid tab module path {module:?}"))?[1..]
                .to_string();
            *module = format!("./{}", bundle_name(&rel));
        }
    }
    let out = serde_json::to_vec_pretty(&installed)
        .map_err(|e| format!("serialize installed manifest: {e}"))?;
    tokio::fs::write(staging.join("objectiveai.json"), out)
        .await
        .map_err(|e| format!("write installed manifest: {e}"))
}

/// The full install pipeline. Lock discipline mirrors the laboratory
/// host's `plugin_image::ensure`: probe → machine-wide lock →
/// re-probe → work → temp cleanup on success AND failure → explicit
/// release on EVERY path (a `LockClaim` drop deliberately does not
/// release).
pub(crate) async fn install(
    app: &tauri::AppHandle,
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
        format!("plugins: {identity}: fetching tag {version}"),
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
        let checkout = objectiveai_sdk::gitrepo::fetch_at_tag(
            &dirs.temp_dir(),
            Some(&dirs.override_dir()),
            &owner,
            &name,
            version,
        )
        .await?;
        let staging = dirs.temp_dir().join(uuid::Uuid::new_v4().to_string());
        let built = build_install(app, &identity, &checkout.dir, &staging).await;
        let landed = match built {
            Ok(()) => async {
                if let Some(parent) = dest.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| format!("create install dir: {e}"))?;
                }
                tokio::fs::rename(&staging, &dest)
                    .await
                    .map_err(|e| format!("land {identity}: {e}"))
            }
            .await,
            Err(e) => Err(e),
        };
        if landed.is_err() {
            objectiveai_sdk::gitrepo::remove_checkout(&staging).await;
        }
        objectiveai_sdk::gitrepo::remove_checkout(&checkout.dir).await;
        landed
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
    match install(&app, &dirs, &owner, &name, &version).await {
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
