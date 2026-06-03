//! Tool discovery on the local filesystem.
//!
//! Tools live at `<base_dir>/tools/` with manifests as `<name>.json`
//! and executables as `<exec>` alongside, both at the same level — no
//! per-tool subdirectory. [`Client::resolve_tool`] reads the manifest
//! and joins `manifest.exec` onto the tools directory to produce the
//! executable path.

use std::path::{Path, PathBuf};

use super::super::Client;
use super::{Manifest, ManifestWithNameAndSource};

/// Two-step parse: try `ManifestWithNameAndSource` first, then fall
/// back to a bare `Manifest` with `name = file_stem` and
/// `source = absolute_path`. Returns `None` on missing / unreadable /
/// malformed files.
async fn parse_manifest_file(path: &Path) -> Option<ManifestWithNameAndSource> {
    let bytes = tokio::fs::read(path).await.ok()?;
    if let Ok(full) =
        serde_json::from_slice::<ManifestWithNameAndSource>(&bytes)
    {
        return Some(full);
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
    let name = path.file_stem()?.to_str()?.to_string();
    let source = path.to_string_lossy().into_owned();
    Some(ManifestWithNameAndSource {
        name,
        manifest,
        source,
    })
}

impl Client {
    /// The tools directory: `<base_dir>/tools`.
    pub fn tools_dir(&self) -> PathBuf {
        self.base_dir().join("tools")
    }

    /// Resolve a tool name to its executable path. Reads the tool's
    /// manifest at `<base_dir>/tools/<name>.json`, joins
    /// `manifest.exec` to the tools directory, and returns the path
    /// if it exists as a regular file. Returns `None` when the
    /// manifest is missing/malformed or the referenced exec is
    /// absent.
    pub async fn resolve_tool(&self, name: &str) -> Option<PathBuf> {
        let bundle = self.get_tool(name).await?;
        let exec_path = self.tools_dir().join(&bundle.manifest.exec);
        if tokio::fs::metadata(&exec_path)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            Some(exec_path)
        } else {
            None
        }
    }

    /// Look up a single tool manifest by name. Reads
    /// `<base_dir>/tools/<name>.json`. Returns `None` on missing /
    /// unreadable / malformed files.
    pub async fn get_tool(
        &self,
        name: &str,
    ) -> Option<ManifestWithNameAndSource> {
        let path = self.tools_dir().join(format!("{name}.json"));
        parse_manifest_file(&path).await
    }

    /// Enumerate tool manifests in the tools directory. Reads each
    /// `.json` file in `<base_dir>/tools/`, deserializes it as a
    /// [`Manifest`], and pairs it with the file's stem (`name`) and
    /// absolute path (`source`). Every failure mode — missing dir,
    /// unreadable file, malformed JSON, missing required field — is
    /// silently skipped; the return type is plain `Vec` rather than
    /// `Result` to reflect that.
    ///
    /// Results are sorted by manifest mtime descending (most recently
    /// modified first), then `skip(offset).take(limit)` is applied —
    /// matching `list_plugins` and the logs list endpoints. Pass
    /// `(0, usize::MAX)` for an unbounded list.
    pub async fn list_tools(
        &self,
        offset: usize,
        limit: usize,
    ) -> Vec<ManifestWithNameAndSource> {
        let dir = self.tools_dir();
        let Ok(mut read_dir) = tokio::fs::read_dir(&dir).await else {
            return Vec::new();
        };
        let mut paths: Vec<PathBuf> = Vec::new();
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                paths.push(path);
            }
        }
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
