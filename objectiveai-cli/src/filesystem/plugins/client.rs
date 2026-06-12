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

use std::path::{Path, PathBuf};

use super::super::Client;
use super::{Manifest, ManifestWithNameAndSource};

/// Parse an on-disk `objectiveai.json` (a bare [`Manifest`]) into a
/// [`ManifestWithNameAndSource`], deriving `name` from the `<name>`
/// path segment (`.../<owner>/<name>/<version>/objectiveai.json`) and
/// `source` from the file path. `None` on missing / unreadable /
/// malformed / invalid files.
async fn parse_manifest_file(path: &Path) -> Option<ManifestWithNameAndSource> {
    let bytes = tokio::fs::read(path).await.ok()?;
    let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
    manifest.validate().ok()?;
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
        let bundle = self.get_plugin(owner, name, version).await?;
        let cli_dir = self.plugin_cli_dir(owner, name, version);
        Some((
            crate::filesystem::tools::platform_exec(&bundle.manifest.exec),
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
    ) -> Option<ManifestWithNameAndSource> {
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
    ) -> Vec<ManifestWithNameAndSource> {
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

impl Client {
    /// Install a plugin from a GitHub repository.
    ///
    /// 1. Fetches `objectiveai.json` from `raw.githubusercontent.com`
    ///    at the supplied `commit_sha` (or the default branch via
    ///    `HEAD` when none).
    /// 2. Parses it as a [`Manifest`].
    /// 3. Downloads the declared release assets from
    ///    `https://github.com/<owner>/<repository>/releases/download/v<version>/<asset>`:
    ///    `cli_zip` (when declared) extracts into
    ///    `<plugin dir>/cli/`, `viewer_zip` into `…/viewer/`.
    ///    Neither is required — a manifest whose exec invokes
    ///    PATH-resolved programs installs with just the manifest.
    ///
    /// `headers` is an optional `IndexMap<String, String>` that gets
    /// attached to every HTTP request (e.g. `Authorization` for
    /// private repos / higher rate limits). The cli always passes
    /// `None`.
    ///
    /// Failures are returned as [`super::InstallError`] wrapped by
    /// [`super::super::Error::Install`]. The `bool` is retained for
    /// wire compatibility and is always `true` on success — the
    /// platform gate that used to yield `Ok(false)` died with the
    /// per-platform binaries map (per-OS support is now expressed by
    /// the exec vectors themselves).
    pub async fn install_plugin(
        &self,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        validate_install_inputs(owner, repository, commit_sha)?;
        let manifest = self
            .fetch_plugin_manifest(owner, repository, commit_sha, headers)
            .await?;
        let source = raw_manifest_url(owner, repository, commit_sha);
        self.install_plugin_from_manifest(
            owner, repository, &manifest, &source, headers, upgrade,
        )
        .await
    }

    /// Step 1 of `install_plugin`: fetch `<owner>/<repo>/<ref>/objectiveai.json`
    /// from `raw.githubusercontent.com` and parse it as a [`Manifest`].
    /// Exposed publicly so callers can inspect the manifest before
    /// committing to an install (e.g. for whitelist checks).
    pub async fn fetch_plugin_manifest(
        &self,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
    ) -> Result<Manifest, super::super::Error> {
        self.fetch_plugin_manifest_impl(
            "https://raw.githubusercontent.com",
            owner,
            repository,
            commit_sha,
            headers,
        )
        .await
    }

    /// Step 2 of `install_plugin`: given an already-parsed manifest,
    /// download its declared release assets (`cli_zip` → `cli/`,
    /// `viewer_zip` → `viewer/`) and persist the manifest.
    pub async fn install_plugin_from_manifest(
        &self,
        owner: &str,
        repository: &str,
        manifest: &Manifest,
        source: &str,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        // `install_plugin_from_manifest` is a public entry — callers
        // may hand us a manifest with no fetch step ever happening, so
        // re-validate inputs here. `install_plugin` already validated
        // before fetching; the second call is cheap and idempotent.
        validate_install_inputs(owner, repository, None)?;
        self.install_from_manifest_impl(
            "https://github.com",
            owner,
            repository,
            manifest,
            source,
            headers,
            upgrade,
        )
        .await
    }

    /// Test-only entry point that exposes the raw / releases URL
    /// bases so in-process mock servers can intercept the requests.
    /// Threads both URLs through the same fetch + install_from path
    /// used by production.
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
        validate_install_inputs(owner, repository, commit_sha)?;
        let manifest = self
            .fetch_plugin_manifest_impl(
                raw_base, owner, repository, commit_sha, headers,
            )
            .await?;
        let reference = commit_sha.unwrap_or("HEAD");
        let source = format!(
            "{raw_base}/{owner}/{repository}/{reference}/objectiveai.json"
        );
        self.install_from_manifest_impl(
            releases_base,
            owner,
            repository,
            &manifest,
            &source,
            headers,
            upgrade,
        )
        .await
    }

    /// Test-only fetch-only entry point, mirrors `install_plugin_at`.
    #[cfg(test)]
    pub(super) async fn fetch_plugin_manifest_at(
        &self,
        raw_base: &str,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
    ) -> Result<Manifest, super::super::Error> {
        self.fetch_plugin_manifest_impl(
            raw_base, owner, repository, commit_sha, headers,
        )
        .await
    }

    async fn fetch_plugin_manifest_impl(
        &self,
        raw_base: &str,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
    ) -> Result<Manifest, super::super::Error> {
        let http = reqwest::Client::new();
        let header_map = build_headers(headers)?;
        let reference = commit_sha.unwrap_or("HEAD");
        let manifest_url = format!(
            "{raw_base}/{owner}/{repository}/{reference}/objectiveai.json"
        );
        let resp = http
            .get(&manifest_url)
            .headers(header_map)
            .send()
            .await
            .map_err(super::InstallError::ManifestRequest)?;
        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(super::InstallError::ManifestResponse)?;
        if !status.is_success() {
            return Err(super::InstallError::ManifestBadStatus {
                code: status,
                url: manifest_url,
                body: String::from_utf8_lossy(&bytes).into_owned(),
            }
            .into());
        }
        let mut de = serde_json::Deserializer::from_slice(&bytes);
        let manifest: Manifest = serde_path_to_error::deserialize(&mut de)
            .map_err(super::InstallError::ManifestParse)?;
        manifest
            .validate()
            .map_err(super::InstallError::ManifestInvalid)?;
        Ok(manifest)
    }

    async fn install_from_manifest_impl(
        &self,
        releases_base: &str,
        owner: &str,
        repository: &str,
        manifest: &Manifest,
        _source: &str,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        // 0. Tool-name budget check. Build the same string
        //    `Manifest::tool_name` materializes (owner-name-version
        //    with `.` -> `-`) and reject if longer than the 100-char
        //    budget we leave under Anthropic's 128-char hard cap.
        let tool_name = manifest.tool_name(repository);
        if tool_name.len() > 100 {
            return Err(super::InstallError::ToolNameTooLong {
                len: tool_name.len(),
                tool_name,
            }
            .into());
        }

        let version = manifest.version.clone();
        let plugin_dir = self.plugin_dir(owner, repository, &version);
        let cli_dir = self.plugin_cli_dir(owner, repository, &version);
        let viewer_dir = plugin_dir.join("viewer");
        let manifest_path = plugin_dir.join("objectiveai.json");

        // 1. Existing-install check: the manifest sibling file is the
        //    source of truth for "this plugin is installed."
        let manifest_exists = tokio::fs::metadata(&manifest_path).await.is_ok();
        if manifest_exists && !upgrade {
            return Err(super::InstallError::AlreadyInstalled {
                repository: repository.to_string(),
            }
            .into());
        }

        // 2. Clean prior install data when --upgrade. Best-effort: any
        //    delete failure surfaces later as a write-phase error
        //    (e.g. ManifestPersist) if the artifact is truly stuck.
        //    Extra entries under <plugin_dir>/ are untouched.
        if upgrade {
            let _ = tokio::fs::remove_file(&manifest_path).await;
            let _ = tokio::fs::remove_dir_all(&cli_dir).await;
            let _ = tokio::fs::remove_dir_all(&viewer_dir).await;
        }

        // 3. Network phase: fetch everything into memory before any
        //    disk write. A network failure here leaves the disk in
        //    whatever state step 2 left it in (empty if upgrade,
        //    unchanged if fresh install — since step 1's check would
        //    have refused).
        let http = reqwest::Client::new();
        let cli_zip_bytes: Option<Vec<u8>> = if let Some(cli_zip_name) =
            &manifest.cli_zip
        {
            let cli_url = format!(
                "{releases_base}/{owner}/{repository}/releases/download/v{version}/{cli_zip_name}",
                version = manifest.version,
            );
            let resp = http
                .get(&cli_url)
                .headers(build_headers(headers)?)
                .send()
                .await
                .map_err(super::InstallError::CliZipRequest)?;
            let status = resp.status();
            if !status.is_success() {
                return Err(super::InstallError::CliZipBadStatus {
                    code: status,
                    url: cli_url,
                }
                .into());
            }
            Some(
                resp.bytes()
                    .await
                    .map_err(super::InstallError::CliZipResponse)?
                    .to_vec(),
            )
        } else {
            None
        };

        let zip_bytes: Option<Vec<u8>> = if let Some(viewer_zip_name) =
            &manifest.viewer_zip
        {
            let viewer_url = format!(
                "{releases_base}/{owner}/{repository}/releases/download/v{version}/{viewer_zip_name}",
                version = manifest.version,
            );
            let resp = http
                .get(&viewer_url)
                .headers(build_headers(headers)?)
                .send()
                .await
                .map_err(super::InstallError::ViewerZipRequest)?;
            let status = resp.status();
            if !status.is_success() {
                return Err(super::InstallError::ViewerZipBadStatus {
                    code: status,
                    url: viewer_url,
                }
                .into());
            }
            Some(
                resp.bytes()
                    .await
                    .map_err(super::InstallError::ViewerZipResponse)?
                    .to_vec(),
            )
        } else {
            None
        };

        let manifest_bytes: Vec<u8> = {
            // Override the author-claimed `owner` with the GitHub
            // `<owner>` we were actually installed from — forks land
            // on disk with the fork's owner, not the upstream's. The
            // on-disk `objectiveai.json` is the bare manifest; `name`
            // / `version` / `owner` are encoded in the directory path.
            let mut manifest = manifest.clone();
            manifest.owner = owner.to_string();
            serde_json::to_vec_pretty(&manifest)
                .map_err(super::InstallError::ManifestSerialize)?
        };

        // 4. Plugin dir setup. Idempotent — preserves any pre-existing
        //    "extra data" the plugin's runtime created.
        tokio::fs::create_dir_all(&plugin_dir).await.map_err(|e| {
            super::InstallError::PluginDirCreate(plugin_dir.clone(), e)
        })?;

        // 5. Concurrent write phase via try_join!. Three branches fan
        //    out, short-circuit on first error.
        tokio::try_join!(
            write_zip_branch(cli_dir, cli_zip_bytes),
            write_zip_branch(viewer_dir, zip_bytes),
            write_manifest_branch(manifest_path, manifest_bytes),
        )?;

        Ok(true)
    }
}

/// Extract a downloaded release zip into `dir` (used for both the
/// `cli/` and `viewer/` bundles). `None` bytes = the manifest didn't
/// declare this asset — no-op.
async fn write_zip_branch(
    dir: PathBuf,
    zip_bytes: Option<Vec<u8>>,
) -> Result<(), super::InstallError> {
    let Some(bytes) = zip_bytes else {
        return Ok(());
    };
    tokio::fs::create_dir_all(&dir).await.map_err(|e| {
        super::InstallError::ZipExtract(dir.clone(), e.to_string())
    })?;
    let dir_for_blocking = dir.clone();
    tokio::task::spawn_blocking(move || {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| format!("zip archive open: {e}"))?;
        archive
            .extract(&dir_for_blocking)
            .map_err(|e| format!("extract: {e}"))
    })
    .await
    .map_err(|e| super::InstallError::ZipExtract(dir.clone(), format!("join: {e}")))?
    .map_err(|e| super::InstallError::ZipExtract(dir.clone(), e))?;
    Ok(())
}

async fn write_manifest_branch(
    manifest_path: PathBuf,
    bytes: Vec<u8>,
) -> Result<(), super::InstallError> {
    tokio::fs::write(&manifest_path, &bytes).await.map_err(|e| {
        super::InstallError::ManifestPersist(manifest_path.clone(), e)
    })
}

/// Reject reserved plugin repository names before any install
/// side-effect. `objectiveai` (case-insensitive) is reserved because
/// the viewer uses it as the Tauri channel name for built-in events;
/// a plugin with that repository name would shadow them.
fn check_repository_name(repository: &str) -> Result<(), super::InstallError> {
    if repository.eq_ignore_ascii_case("objectiveai") {
        return Err(super::InstallError::ReservedRepositoryName {
            repository: repository.to_string(),
        });
    }
    Ok(())
}

/// Identifier shape check shared by `owner`, `repository`, and
/// `commit`: Anthropic's tool-name regex (`^[a-zA-Z0-9_-]{1,128}$`)
/// plus `.` (so semver-shaped versions and dotted commit refs flow
/// through cleanly; the `.` -> `-` substitution happens when the tool
/// name is materialized via [`super::Manifest::tool_name`]).
fn validate_identifier(
    kind: &'static str,
    value: &str,
) -> Result<(), super::InstallError> {
    let valid_len = !value.is_empty() && value.len() <= 128;
    let valid_chars = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if !valid_len || !valid_chars {
        return Err(super::InstallError::InvalidIdentifier {
            kind,
            value: value.to_string(),
        });
    }
    Ok(())
}

/// Combined shape check for the three caller-supplied identifiers
/// every install entry point takes. Used by `install_plugin`,
/// `install_plugin_from_manifest`, and the `#[cfg(test)]`
/// `install_plugin_at`. Calls [`check_repository_name`] first so a
/// reserved-name failure takes precedence over a generic regex
/// failure for the same input.
fn validate_install_inputs(
    owner: &str,
    repository: &str,
    commit_sha: Option<&str>,
) -> Result<(), super::InstallError> {
    check_repository_name(repository)?;
    validate_identifier("owner", owner)?;
    validate_identifier("repository", repository)?;
    if let Some(sha) = commit_sha {
        validate_identifier("commit", sha)?;
    }
    Ok(())
}

/// Convention: the raw-GitHub URL we'd fetch `objectiveai.json` from
/// for a given (owner, repository, optional commit sha). Defaults to
/// `HEAD` when no commit is supplied. Lifted out so the cli and the
/// SDK's own `install_plugin` wrapper share one source of truth.
pub fn raw_manifest_url(
    owner: &str,
    repository: &str,
    commit_sha: Option<&str>,
) -> String {
    let reference = commit_sha.unwrap_or("HEAD");
    format!(
        "https://raw.githubusercontent.com/{owner}/{repository}/{reference}/objectiveai.json"
    )
}

pub(super) fn build_headers(
    headers: Option<&indexmap::IndexMap<String, String>>,
) -> Result<reqwest::header::HeaderMap, super::InstallError> {
    let mut out = reqwest::header::HeaderMap::new();
    let Some(h) = headers else {
        return Ok(out);
    };
    for (k, v) in h {
        let name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
            .map_err(|e| super::InstallError::InvalidHeaderName {
                name: k.clone(),
                reason: e.to_string(),
            })?;
        let value = reqwest::header::HeaderValue::from_str(v).map_err(|e| {
            super::InstallError::InvalidHeaderValue {
                name: k.clone(),
                reason: e.to_string(),
            }
        })?;
        out.insert(name, value);
    }
    Ok(out)
}
