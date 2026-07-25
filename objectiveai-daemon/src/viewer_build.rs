//! The plugin viewer-extension BUILD pipeline: turn a checked-out
//! plugin repo's declared `viewer` root into the final version-dir
//! layout — rewritten `objectiveai.json` + self-contained ESM tab
//! bundles under `viewer/` + the icon — assembled in a caller-owned
//! staging dir.
//!
//! The daemon is the system's build machine: the
//! `GET /plugins/{owner}/{name}/{version}/viewer` route
//! ([`crate::http::plugin_routes`]) runs this and streams the staging
//! dir back as tar.gz. (The viewer currently carries a local copy of
//! this pipeline for its on-device installs; that copy dies when the
//! daemon-client consolidation points it at the route — THIS module
//! is the authoritative implementation.) Fetch happens elsewhere
//! ([`objectiveai_sdk::gitrepo`]); this module owns everything from
//! checkout to finished artifact.
//!
//! The pipeline: parse + validate the manifest's viewer half →
//! rewrite `package.json` in place (strip host-provided packages,
//! inject the host-pinned esbuild) → `pnpm install` → esbuild-bundle
//! every declared tab module (react + subpaths external — the host
//! viewer serves them) → copy the icon → write the rewritten
//! manifest. Everything validates in temp and fails loudly; on `Err`,
//! staging is the caller's to sweep.
//!
//! Machine requirements: pnpm resolvable on PATH (which implies Node
//! — esbuild's `.bin` entry is a Node wrapper too). esbuild itself
//! arrives via the injected devDependency; a bundled pnpm is planned.

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
/// build owns the bundling contract, so the author's pin (if any) is
/// overwritten.
const ESBUILD_VERSION: &str = "^0.25.0";

/// pnpm as spawned: Windows' CreateProcess won't resolve the `.cmd`
/// shim from a bare `pnpm`.
const PNPM: &str = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };

/// Canonicalize a manifest module/icon path (authored relative to the
/// viewer root, CWD-style) into the uniform root-relative form: `./`
/// stripped, stored with a leading `/` whose root IS the plugin's
/// viewer dir. `None` = a path that tries to leave the root.
fn normalize_module_path(path: &str) -> Option<String> {
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

/// Build a checked-out plugin repo's viewer extension into `staging`,
/// shaped as the final installed version dir: rewritten
/// `objectiveai.json` (fixed `"viewer"` layout, modules remapped
/// source → bundle) + `viewer/` bundles + icon. Everything validates
/// in temp and fails loudly; on `Err`, staging is the caller's to
/// sweep.
pub(crate) async fn build(checkout: &Path, staging: &Path) -> Result<(), String> {
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
        return Err(format!("viewer root {viewer_rel:?} has no package.json"));
    }
    rewrite_package_json(&package_json).await?;
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
        let rel = normalize_module_path(module)
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
        let rel = normalize_module_path(icon)
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

    // The FINAL manifest: fixed `viewer` layout, modules remapped
    // source → bundle. (Typed re-serialize — unmodeled fields drop;
    // readers only consume typed fields.)
    let mut installed = manifest.clone();
    installed.viewer = Some("viewer".to_string());
    if let Some(tabs) = installed.tabs.as_mut() {
        for tab in tabs {
            let module = match tab {
                ViewerTab::Channel { module, .. } | ViewerTab::Tab { module, .. } => module,
            };
            let rel = normalize_module_path(module)
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
