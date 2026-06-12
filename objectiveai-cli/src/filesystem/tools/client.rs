//! Tool discovery on the local filesystem.
//!
//! Tools live at `<bin_dir>/tools/<owner>/<name>/<version>/` (machine-
//! wide, shared by every state) with the
//! manifest as `objectiveai.json` inside the version folder. The
//! manifest's `exec` is a per-OS command vector; at run time the
//! current platform's vector is appended with the caller's args and
//! invoked with CWD = the version folder.

use std::path::{Path, PathBuf};

use super::super::Client;
use super::{Exec, Manifest, ManifestWithNameAndSource};

/// Parse an on-disk `objectiveai.json` (a bare [`Manifest`]) into a
/// [`ManifestWithNameAndSource`], deriving `name` from the `<name>`
/// path segment (`.../<owner>/<name>/<version>/objectiveai.json`) and
/// `source` from the file path. `None` on missing / unreadable /
/// malformed files.
async fn parse_manifest_file(path: &Path) -> Option<ManifestWithNameAndSource> {
    let bytes = tokio::fs::read(path).await.ok()?;
    let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
    // path = .../<owner>/<name>/<version>/objectiveai.json
    // parent = <version>, parent.parent = <name>.
    let name = path.parent()?.parent()?.file_name()?.to_str()?.to_string();
    let source = path.to_string_lossy().into_owned();
    Some(ManifestWithNameAndSource {
        name,
        manifest,
        source,
    })
}

/// The current platform's exec vector from a per-OS [`Exec`]. Shared
/// with the plugins tier, whose manifests carry the same `Exec` type.
pub(crate) fn platform_exec(exec: &Exec) -> Vec<String> {
    if cfg!(target_os = "windows") {
        exec.windows.clone()
    } else if cfg!(target_os = "macos") {
        exec.macos.clone()
    } else {
        exec.linux.clone()
    }
}

impl Client {
    /// The tools directory: `<bin_dir>/tools`.
    pub fn tools_dir(&self) -> PathBuf {
        self.bin_dir().join("tools")
    }

    /// A tool's version directory:
    /// `<base_dir>/tools/<owner>/<name>/<version>`.
    pub fn tool_dir(&self, owner: &str, name: &str, version: &str) -> PathBuf {
        self.tools_dir().join(owner).join(name).join(version)
    }

    /// Resolve a tool coordinate to its `(exec_vector, cwd)` for the
    /// current platform. `cwd` is the version folder; `exec_vector` is
    /// the manifest's per-OS command (possibly empty when the tool
    /// declares no command for this platform — the caller treats that
    /// as an error). `None` when the manifest is missing/malformed.
    pub async fn resolve_tool(
        &self,
        owner: &str,
        name: &str,
        version: &str,
    ) -> Option<(Vec<String>, PathBuf)> {
        let bundle = self.get_tool(owner, name, version).await?;
        let dir = self.tool_dir(owner, name, version);
        Some((platform_exec(&bundle.manifest.exec), dir))
    }

    /// Look up a single tool manifest by coordinate. Reads
    /// `<base_dir>/tools/<owner>/<name>/<version>/objectiveai.json`.
    /// `None` on missing / unreadable / malformed files.
    pub async fn get_tool(
        &self,
        owner: &str,
        name: &str,
        version: &str,
    ) -> Option<ManifestWithNameAndSource> {
        let path = self.tool_dir(owner, name, version).join("objectiveai.json");
        parse_manifest_file(&path).await
    }

    /// Enumerate tool manifests by walking the
    /// `tools/<owner>/<name>/<version>/objectiveai.json` tree. Every
    /// failure mode — missing dir, unreadable file, malformed JSON — is
    /// silently skipped.
    ///
    /// Results are sorted by manifest mtime descending, then
    /// `skip(offset).take(limit)` is applied — matching `list_plugins`.
    /// Pass `(0, usize::MAX)` for an unbounded list.
    pub async fn list_tools(
        &self,
        offset: usize,
        limit: usize,
    ) -> Vec<ManifestWithNameAndSource> {
        let paths = collect_manifest_paths(self.tools_dir()).await;
        let futures = paths.into_iter().map(|p| async move {
            let bundle = parse_manifest_file(&p).await?;
            let modified = tokio::fs::metadata(&p)
                .await
                .ok()?
                .modified()
                .ok()?
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some((modified, bundle))
        });
        let mut entries: Vec<(u64, ManifestWithNameAndSource)> =
            futures::future::join_all(futures)
                .await
                .into_iter()
                .flatten()
                .collect();
        entries.sort_by(|a, b| b.0.cmp(&a.0));
        let iter = entries.into_iter().map(|(_, m)| m);
        if offset > 0 || limit < usize::MAX {
            iter.skip(offset).take(limit).collect()
        } else {
            iter.collect()
        }
    }
}

/// Walk `<root>/<owner>/<name>/<version>/objectiveai.json` and collect
/// every existing manifest file path. Shared shape with the plugins
/// tier. Any non-directory / unreadable level is skipped.
pub(crate) async fn collect_manifest_paths(root: PathBuf) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let Ok(mut owners) = tokio::fs::read_dir(&root).await else {
        return out;
    };
    while let Ok(Some(owner_e)) = owners.next_entry().await {
        let Ok(mut names) = tokio::fs::read_dir(owner_e.path()).await else {
            continue;
        };
        while let Ok(Some(name_e)) = names.next_entry().await {
            let Ok(mut versions) = tokio::fs::read_dir(name_e.path()).await else {
                continue;
            };
            while let Ok(Some(ver_e)) = versions.next_entry().await {
                let manifest = ver_e.path().join("objectiveai.json");
                if tokio::fs::metadata(&manifest)
                    .await
                    .map(|m| m.is_file())
                    .unwrap_or(false)
                {
                    out.push(manifest);
                }
            }
        }
    }
    out
}
