//! The plugin manifest: `objectiveai.json` at the root of a plugin's
//! repo checkout.
//!
//! The container-image successor to the retired zip-based manifest —
//! GCP-Cloud-Build style: the manifest names the build file, the repo
//! IS the build context. Read exclusively from the host's own
//! checkout, never fetched over HTTP and never crossing a wire —
//! which is why the type lives here and not in the SDK.

use std::path::{Path, PathBuf};

/// The manifest file name, at the checkout root.
pub const MANIFEST_FILE: &str = "objectiveai.json";

#[derive(serde::Deserialize)]
pub struct PluginManifest {
    /// Repo-relative path (forward slashes) to the Containerfile /
    /// Dockerfile the plugin's image builds from.
    pub containerfile: String,
    /// The port the plugin's MCP server listens on inside the
    /// container — published to a random loopback host port at create.
    pub port: u16,
}

/// Read + validate `objectiveai.json` at `repo_root`. Returns the
/// manifest and the RESOLVED absolute containerfile path (validated to
/// stay inside the repo and to exist).
pub async fn read(repo_root: &Path) -> Result<(PluginManifest, PathBuf), String> {
    let manifest_path = repo_root.join(MANIFEST_FILE);
    let text = tokio::fs::read_to_string(&manifest_path)
        .await
        .map_err(|e| format!("plugin manifest: read {MANIFEST_FILE}: {e}"))?;
    let manifest: PluginManifest = serde_json::from_str(&text)
        .map_err(|e| format!("plugin manifest: parse {MANIFEST_FILE}: {e}"))?;
    if manifest.port == 0 {
        return Err("plugin manifest: `port` cannot be 0".to_string());
    }
    let containerfile = manifest.containerfile.trim();
    if containerfile.is_empty() {
        return Err("plugin manifest: `containerfile` cannot be empty".to_string());
    }
    // Repo-relative, forward slashes, no traversal: the path joins
    // under the checkout root and must never escape it.
    if containerfile.contains('\\') {
        return Err(
            "plugin manifest: `containerfile` must use forward slashes".to_string()
        );
    }
    if containerfile.starts_with('/') || containerfile.contains(':') {
        return Err(
            "plugin manifest: `containerfile` must be a repo-relative path".to_string()
        );
    }
    if containerfile
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(format!(
            "plugin manifest: `containerfile` has an invalid path component: {containerfile:?}",
        ));
    }
    let resolved = containerfile
        .split('/')
        .fold(repo_root.to_path_buf(), |path, component| {
            path.join(component)
        });
    match tokio::fs::metadata(&resolved).await {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            return Err(format!(
                "plugin manifest: `containerfile` {containerfile:?} is not a file",
            ));
        }
        Err(e) => {
            return Err(format!(
                "plugin manifest: `containerfile` {containerfile:?}: {e}",
            ));
        }
    }
    Ok((manifest, resolved))
}
