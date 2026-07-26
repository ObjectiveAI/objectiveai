//! Reading + validating the plugin manifest (`objectiveai.json` at
//! the root of a plugin's repo checkout) for the CONTAINER builds —
//! GCP-Cloud-Build style: the manifest names the build file, and that
//! file's OWN DIRECTORY is the build context.
//!
//! The TYPE is the SDK's [`objectiveai_sdk::cli::plugins::Manifest`]
//! — the one schema for the one file (the viewer reads its surface
//! half from the same type). This module owns only the host-side
//! VALIDATION, shared by BOTH halves: a manifest declares at least one
//! of them, and a declared containerfile path must stay inside the
//! checkout and exist.

use std::path::{Path, PathBuf};

use objectiveai_sdk::cli::plugins::Manifest;

/// The manifest file name, at the checkout root.
pub const MANIFEST_FILE: &str = "objectiveai.json";

/// A resolved containerfile and the context it builds in.
pub struct BuildFile {
    /// The absolute Containerfile path.
    pub containerfile: PathBuf,
    /// The build CONTEXT — the containerfile's own directory. Only
    /// this subtree is visible to its `COPY` steps, so a plugin can
    /// scope a build to `viewer/` just by putting the file there.
    pub context: PathBuf,
}

/// Read + validate `objectiveai.json` at `repo_root`. Structural
/// validation only ([`Manifest::validate`] — at least one half); each
/// half's containerfile is resolved on demand by
/// [`resolve_build_file`], since a plugin legitimately declares only
/// one of them.
pub async fn read(repo_root: &Path) -> Result<Manifest, String> {
    let manifest_path = repo_root.join(MANIFEST_FILE);
    let text = tokio::fs::read_to_string(&manifest_path)
        .await
        .map_err(|e| format!("plugin manifest: read {MANIFEST_FILE}: {e}"))?;
    let manifest: Manifest = serde_json::from_str(&text)
        .map_err(|e| format!("plugin manifest: parse {MANIFEST_FILE}: {e}"))?;
    manifest
        .validate()
        .map_err(|e| format!("plugin manifest: {e}"))?;
    Ok(manifest)
}

/// Resolve one half's declared containerfile against the checkout:
/// repo-relative, forward slashes, no traversal, and it must exist.
/// `what` names the field for the error messages (`mcp.containerfile`
/// / `viewer.containerfile`).
pub async fn resolve_build_file(
    repo_root: &Path,
    declared: &str,
    what: &str,
) -> Result<BuildFile, String> {
    let declared = declared.trim();
    if declared.is_empty() {
        return Err(format!("plugin manifest: `{what}` cannot be empty"));
    }
    // Repo-relative, forward slashes, no traversal: the path joins
    // under the checkout root and must never escape it.
    if declared.contains('\\') {
        return Err(format!(
            "plugin manifest: `{what}` must use forward slashes"
        ));
    }
    if declared.starts_with('/') || declared.contains(':') {
        return Err(format!(
            "plugin manifest: `{what}` must be a repo-relative path"
        ));
    }
    if declared
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(format!(
            "plugin manifest: `{what}` has an invalid path component: {declared:?}",
        ));
    }
    let containerfile = declared
        .split('/')
        .fold(repo_root.to_path_buf(), |path, component| {
            path.join(component)
        });
    match tokio::fs::metadata(&containerfile).await {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            return Err(format!(
                "plugin manifest: `{what}` {declared:?} is not a file",
            ));
        }
        Err(e) => {
            return Err(format!("plugin manifest: `{what}` {declared:?}: {e}"));
        }
    }
    // The containerfile's own directory is the context. It always has
    // a parent: the path resolved under `repo_root`, so at worst the
    // file sits at the checkout root and the context IS the checkout.
    let context = containerfile
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| repo_root.to_path_buf());
    Ok(BuildFile {
        containerfile,
        context,
    })
}
