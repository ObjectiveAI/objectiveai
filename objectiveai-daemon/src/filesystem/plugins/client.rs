//! Plugin discovery on the local filesystem.
//!
//! Installed plugins live at
//! `<base_dir>/plugins/<owner>/<name>/<version>/`, with the cli-side
//! payload at `…/cli/` (the exec working directory, extracted from
//! the manifest's `cli_zip` when one is declared), the optional
//! viewer bundle at `…/viewer/`, and the manifest as
//! `…/objectiveai.json` inside the version folder. The cli's
//! `plugins run` dispatch uses [`Client::resolve_plugin`] to turn an
//! `(owner, name, version)` coordinate into the platform's exec
//! vector plus that `cli/` working directory — the same model tools
//! use, with the extra `cli` folder.
//!
//! The GitHub-install pipeline itself lives in
//! [`crate::filesystem::install`]; the `install_plugin*` methods here
//! are thin wrappers that drive it with `plugins::Manifest`.

use std::path::{Path, PathBuf};

use super::super::Client;
use super::Manifest;
use crate::filesystem::install::{
    ExtraAsset, InstallKind, InstallManifest, platform_cli_zip,
    validate_install_inputs,
};

/// Parse an on-disk `objectiveai.json` into a [`Manifest`] (and
/// [`Manifest::validate`] it). The manifest declares its own `owner` /
/// `name` / `version`; nothing is derived from the path. `None` on
/// missing / unreadable / malformed / invalid files.
async fn parse_manifest_file(path: &Path) -> Option<Manifest> {
    let bytes = tokio::fs::read(path).await.ok()?;
    let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
    manifest.validate().ok()?;
    Some(manifest)
}

/// Walk `<root>/<owner>/<name>/<version>/objectiveai.json` and collect
/// every existing manifest file path. Any non-directory / unreadable
/// level is skipped.
async fn collect_manifest_paths(root: PathBuf) -> Vec<PathBuf> {
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

impl Client {
    /// The plugins directory: `<bin_dir>/plugins` — installed
    /// plugins are machine-wide, shared by every state.
    pub fn plugins_dir(&self) -> PathBuf {
        self.bin_dir().join("plugins")
    }

    /// The directory that holds a plugin's installed artifacts:
    /// `<plugins_dir>/<owner>/<name>/<version>/`. Contains the
    /// manifest `objectiveai.json`, the `cli/` exec working
    /// directory, and an optional `viewer/` bundle.
    pub fn plugin_dir(&self, owner: &str, name: &str, version: &str) -> PathBuf {
        self.plugins_dir().join(owner).join(name).join(version)
    }

    /// A plugin's cli working directory: `<plugin_dir>/cli/`. The
    /// manifest's exec runs with this as CWD; `cli_zip` extracts
    /// into it at install time.
    pub fn plugin_cli_dir(&self, owner: &str, name: &str, version: &str) -> PathBuf {
        self.plugin_dir(owner, name, version).join("cli")
    }

    /// Resolve a plugin coordinate to its `(exec_vector, cli_dir)`
    /// for the current platform — the same contract
    /// [`Client::resolve_tool`](crate::filesystem::Client::resolve_tool)
    /// has, with the plugin's `cli/` folder as the working directory.
    /// `exec_vector` may be empty when the manifest declares no
    /// command for this platform (viewer-only plugins; the caller
    /// treats that as an error). `None` when the manifest is
    /// missing/malformed/invalid.
    pub async fn resolve_plugin(
        &self,
        owner: &str,
        name: &str,
        version: &str,
    ) -> Option<(Vec<String>, PathBuf)> {
        let manifest = self.get_plugin(owner, name, version).await?;
        let cli_dir = self.plugin_cli_dir(owner, name, version);
        Some((
            crate::filesystem::tools::platform_exec(&manifest.exec),
            cli_dir,
        ))
    }

    /// Look up a single plugin manifest by coordinate. Reads
    /// `<base_dir>/plugins/<owner>/<name>/<version>/objectiveai.json`.
    /// Returns `None` if the file is missing, unreadable, malformed, or
    /// invalid.
    pub async fn get_plugin(
        &self,
        owner: &str,
        name: &str,
        version: &str,
    ) -> Option<Manifest> {
        let path = self
            .plugin_dir(owner, name, version)
            .join("objectiveai.json");
        parse_manifest_file(&path).await
    }

    /// Enumerate plugin manifests by walking the
    /// `plugins/<owner>/<name>/<version>/objectiveai.json` tree. Every
    /// failure mode — missing dir, unreadable file, malformed JSON,
    /// missing required field — is silently skipped; the return type is
    /// plain `Vec` rather than `Result` to reflect that.
    ///
    /// Results are sorted by manifest mtime descending (most recently
    /// modified first), then `skip(offset).take(limit)` is applied —
    /// matching the convention of the logs list endpoints. Pass
    /// `(0, usize::MAX)` for an unbounded list.
    ///
    /// The directory walk is sequential but per-file read+parse runs
    /// concurrently via [`futures::future::join_all`].
    pub async fn list_plugins(
        &self,
        offset: usize,
        limit: usize,
    ) -> Vec<Manifest> {
        let paths = collect_manifest_paths(self.plugins_dir()).await;
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
        let mut entries: Vec<(u64, Manifest)> =
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

impl Client {
    /// Install a plugin from a GitHub repository: fetch
    /// `objectiveai.json`, download the current platform's `cli_zip`
    /// plus the `viewer_zip` (when declared), extract them, and persist
    /// the manifest verbatim. Thin wrapper over the shared
    /// [`Client::install_from_github_at`] engine. `headers` attaches to
    /// every request (e.g. `Authorization`); the cli passes `None`. The
    /// `bool` is always `true` on success (retained for wire compat).
    pub async fn install_plugin(
        &self,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        self.install_from_github_at::<Manifest>(
            "https://raw.githubusercontent.com",
            "https://github.com",
            owner,
            repository,
            commit_sha,
            headers,
            upgrade,
        )
        .await
    }

    /// Fetch + parse `<owner>/<repo>/<ref>/objectiveai.json`. Exposed so
    /// callers can inspect the manifest before committing to an install
    /// (e.g. for whitelist checks).
    pub async fn fetch_plugin_manifest(
        &self,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
    ) -> Result<Manifest, super::super::Error> {
        self.fetch_manifest_at::<Manifest>(
            "https://raw.githubusercontent.com",
            owner,
            repository,
            commit_sha,
            headers,
        )
        .await
    }

    /// Given an already-parsed manifest, download its release assets and
    /// persist it. `source` is retained for API compatibility and is
    /// unused — the engine derives every path from the coordinate.
    pub async fn install_plugin_from_manifest(
        &self,
        owner: &str,
        repository: &str,
        manifest: &Manifest,
        source: &str,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        let _ = source;
        // Public entry — callers may hand us a manifest with no fetch
        // ever happening, so validate inputs here (cheap + idempotent).
        validate_install_inputs(InstallKind::Plugin, owner, repository, None)?;
        self.install_parsed_at::<Manifest>(
            "https://github.com",
            owner,
            repository,
            manifest,
            headers,
            upgrade,
        )
        .await
    }

    /// Test-only entry threading mock URL bases through the engine.
    #[cfg(test)]
    pub(super) async fn install_plugin_at(
        &self,
        raw_base: &str,
        releases_base: &str,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        self.install_from_github_at::<Manifest>(
            raw_base,
            releases_base,
            owner,
            repository,
            commit_sha,
            headers,
            upgrade,
        )
        .await
    }

    /// Test-only fetch-only entry, mirrors `install_plugin_at`.
    #[cfg(test)]
    pub(super) async fn fetch_plugin_manifest_at(
        &self,
        raw_base: &str,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
    ) -> Result<Manifest, super::super::Error> {
        self.fetch_manifest_at::<Manifest>(
            raw_base, owner, repository, commit_sha, headers,
        )
        .await
    }
}

/// Plugins install via the shared engine: cli bundle + a viewer bundle
/// (when `viewer_zip` is declared), under the `plugins/` dir tree.
impl InstallManifest for Manifest {
    const KIND: InstallKind = InstallKind::Plugin;
    fn validate_manifest(&self) -> Result<(), &'static str> {
        self.validate()
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn cli_zip_for_platform(&self) -> Option<&str> {
        platform_cli_zip(&self.cli_zip)
    }
    fn extra_assets(&self) -> Vec<ExtraAsset> {
        match &self.viewer_zip {
            Some(filename) => vec![ExtraAsset {
                filename: filename.clone(),
                subdir: "viewer",
            }],
            None => Vec::new(),
        }
    }
}
