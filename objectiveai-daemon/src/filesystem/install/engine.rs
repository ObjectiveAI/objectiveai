//! The generic GitHub-install engine shared by tools and plugins.
//!
//! Both kinds fetch `objectiveai.json` from `raw.githubusercontent.com`,
//! download the current platform's `cli_zip` release asset (plus any
//! extra assets — plugins add a `viewer_zip`), extract them into the
//! install's version folder, and write the manifest verbatim. The
//! per-kind differences are abstracted behind [`InstallManifest`]; the
//! thin `install_{tool,plugin}*` wrappers (in `tools/install.rs` and
//! `plugins/client.rs`) drive this engine.

use std::path::PathBuf;

use super::{InstallError, build_headers, validate_install_inputs};
use crate::filesystem::Client;
use crate::filesystem::plugins::CliZip;

/// Which install kind a manifest belongs to — selects the on-disk dir
/// A non-cli release zip the engine fetches and extracts: the asset
/// filename (from the manifest) plus the subdir of the version folder
/// it extracts into. Plugins yield one of these for `viewer_zip` →
/// `"viewer"`; tools yield none.
pub(crate) struct ExtraAsset {
    pub filename: String,
    pub subdir: &'static str,
}

/// The install-relevant surface a manifest exposes to the engine.
/// Implemented by `plugins::Manifest`.
pub(crate) trait InstallManifest:
    serde::Serialize + serde::de::DeserializeOwned + Send
{
    /// Validate fields serde alone can't enforce. Called after parse.
    /// (Named distinctly from the manifests' inherent `validate` so impl
    /// bodies can delegate to that inherent method unambiguously.)
    fn validate_manifest(&self) -> Result<(), &'static str>;
    fn version(&self) -> &str;
    /// The current platform's cli-bundle asset filename, if declared.
    fn cli_zip_for_platform(&self) -> Option<&str>;
    /// Extra (non-cli) release zips to fetch + extract.
    fn extra_assets(&self) -> Vec<ExtraAsset>;
}

/// The current platform's entry from a [`CliZip`] — matched on OS *and*
/// CPU architecture. Shared by both manifest impls so the
/// `cfg!(target_os)` / `cfg!(target_arch)` ladders live in one place. An
/// arch other than `x86_64` / `aarch64` (or an absent entry) yields
/// `None` — nothing to fetch.
pub(crate) fn platform_cli_zip(cli_zip: &CliZip) -> Option<&str> {
    let per_os = if cfg!(target_os = "windows") {
        &cli_zip.windows
    } else if cfg!(target_os = "macos") {
        &cli_zip.macos
    } else {
        &cli_zip.linux
    };
    if cfg!(target_arch = "x86_64") {
        per_os.x86_64.as_deref()
    } else if cfg!(target_arch = "aarch64") {
        per_os.aarch64.as_deref()
    } else {
        None
    }
}

/// Which release-asset role a download is for — selects the matching
/// `InstallError` trio. The cli bundle is universal; the viewer bundle
/// is plugin-only.
#[derive(Clone, Copy)]
enum AssetSlot {
    Cli,
    Viewer,
}

impl AssetSlot {
    fn request_err(self, e: reqwest::Error) -> InstallError {
        match self {
            AssetSlot::Cli => InstallError::CliZipRequest(e),
            AssetSlot::Viewer => InstallError::ViewerZipRequest(e),
        }
    }
    fn bad_status_err(self, code: reqwest::StatusCode, url: String) -> InstallError {
        match self {
            AssetSlot::Cli => InstallError::CliZipBadStatus { code, url },
            AssetSlot::Viewer => InstallError::ViewerZipBadStatus { code, url },
        }
    }
    fn response_err(self, e: reqwest::Error) -> InstallError {
        match self {
            AssetSlot::Cli => InstallError::CliZipResponse(e),
            AssetSlot::Viewer => InstallError::ViewerZipResponse(e),
        }
    }
}

fn asset_url(
    releases_base: &str,
    owner: &str,
    repository: &str,
    version: &str,
    filename: &str,
) -> String {
    format!(
        "{releases_base}/{owner}/{repository}/releases/download/v{version}/{filename}"
    )
}

/// GET a release zip into memory, mapping failures to `slot`'s error
/// trio.
async fn download_zip(
    http: &reqwest::Client,
    url: String,
    header_map: reqwest::header::HeaderMap,
    slot: AssetSlot,
) -> Result<Vec<u8>, InstallError> {
    let resp = http
        .get(&url)
        .headers(header_map)
        .send()
        .await
        .map_err(|e| slot.request_err(e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(slot.bad_status_err(status, url));
    }
    Ok(resp.bytes().await.map_err(|e| slot.response_err(e))?.to_vec())
}

impl Client {
    /// `(version_dir, cli_dir)` for the given plugin coordinate.
    fn install_dirs(
        &self,
        owner: &str,
        repository: &str,
        version: &str,
    ) -> (PathBuf, PathBuf) {
        (
            self.plugin_dir(owner, repository, version),
            self.plugin_cli_dir(owner, repository, version),
        )
    }

    /// Fetch `<raw_base>/<owner>/<repo>/<ref>/objectiveai.json`, parse
    /// it as `M`, and [`InstallManifest::validate`] it.
    pub(crate) async fn fetch_manifest_at<M: InstallManifest>(
        &self,
        raw_base: &str,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
    ) -> Result<M, crate::filesystem::Error> {
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
            .map_err(InstallError::ManifestRequest)?;
        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(InstallError::ManifestResponse)?;
        if !status.is_success() {
            return Err(InstallError::ManifestBadStatus {
                code: status,
                url: manifest_url,
                body: String::from_utf8_lossy(&bytes).into_owned(),
            }
            .into());
        }
        let mut de = serde_json::Deserializer::from_slice(&bytes);
        let manifest: M = serde_path_to_error::deserialize(&mut de)
            .map_err(InstallError::ManifestParse)?;
        manifest
            .validate_manifest()
            .map_err(InstallError::ManifestInvalid)?;
        Ok(manifest)
    }

    /// Given an already-parsed manifest, download its release assets and
    /// persist it. The whitelist check (when applicable) belongs
    /// *between* fetch and this call, so this is a distinct entry point.
    pub(crate) async fn install_parsed_at<M: InstallManifest>(
        &self,
        releases_base: &str,
        owner: &str,
        repository: &str,
        manifest: &M,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, crate::filesystem::Error> {
        let version = manifest.version();
        let (version_dir, cli_dir) =
            self.install_dirs(owner, repository, version);
        let manifest_path = version_dir.join("objectiveai.json");

        // 1. Existing-install check: the manifest sibling file is the
        //    source of truth for "this is installed."
        let manifest_exists = tokio::fs::metadata(&manifest_path).await.is_ok();
        if manifest_exists && !upgrade {
            return Err(InstallError::AlreadyInstalled {
                repository: repository.to_string(),
            }
            .into());
        }

        // 2. Network phase: fetch everything into memory before any disk
        //    write. A network failure here leaves the disk untouched —
        //    an existing install (upgrade path) keeps working until the
        //    write phase actually begins.
        let http = reqwest::Client::new();
        let header_map = build_headers(headers)?;

        let cli_bytes: Option<Vec<u8>> =
            if let Some(name) = manifest.cli_zip_for_platform() {
                let url =
                    asset_url(releases_base, owner, repository, version, name);
                Some(
                    download_zip(&http, url, header_map.clone(), AssetSlot::Cli)
                        .await?,
                )
            } else {
                None
            };

        // Extra assets (plugins: the viewer bundle; tools: none). Pair
        // each with its target subdir under the version folder.
        let mut extra_branches: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
        for extra in manifest.extra_assets() {
            let url = asset_url(
                releases_base,
                owner,
                repository,
                version,
                &extra.filename,
            );
            let bytes =
                download_zip(&http, url, header_map.clone(), AssetSlot::Viewer)
                    .await?;
            extra_branches.push((version_dir.join(extra.subdir), Some(bytes)));
        }

        // The fetched manifest is written to disk verbatim — exactly as
        // authored, `owner` included. (The install coordinate is still
        // encoded in the directory path; we don't rewrite the body.)
        let manifest_bytes: Vec<u8> = serde_json::to_vec_pretty(manifest)
            .map_err(InstallError::ManifestSerialize)?;

        // 3. Pre-write clean. The manifest is the installed-ness commit
        //    gate, so it is removed FIRST — from here until step 5
        //    lands, the install reads as "not installed", and a process
        //    killed in between leaves a retryable not-installed state
        //    rather than an installed-but-broken one. The zip target
        //    dirs are cleared so an extract can never mix two releases'
        //    trees. Extra entries under the version dir are untouched.
        let _ = tokio::fs::remove_file(&manifest_path).await;
        let _ = tokio::fs::remove_dir_all(&cli_dir).await;
        for (dir, _) in &extra_branches {
            let _ = tokio::fs::remove_dir_all(dir).await;
        }
        tokio::fs::create_dir_all(&version_dir).await.map_err(|e| {
            InstallError::PluginDirCreate(version_dir.clone(), e)
        })?;

        // 4. Zip extraction — all bundles concurrently (disjoint trees,
        //    none is the commit gate).
        let mut branches = Vec::with_capacity(1 + extra_branches.len());
        branches.push(write_zip_branch(cli_dir, cli_bytes));
        for (dir, bytes) in extra_branches {
            branches.push(write_zip_branch(dir, bytes));
        }
        futures::future::try_join_all(branches).await?;

        // 5. Manifest LAST, atomically — the commit gate. Only a
        //    fully-extracted install ever becomes visible.
        write_manifest_branch(manifest_path, manifest_bytes).await?;

        Ok(true)
    }

    /// Full GitHub install: validate inputs, fetch + parse the manifest,
    /// then download assets and persist. URL bases are parameters so
    /// in-process mock servers can intercept; the public wrappers bake
    /// in the production bases.
    pub(crate) async fn install_from_github_at<M: InstallManifest>(
        &self,
        raw_base: &str,
        releases_base: &str,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, crate::filesystem::Error> {
        validate_install_inputs(owner, repository, commit_sha)?;
        let manifest = self
            .fetch_manifest_at::<M>(raw_base, owner, repository, commit_sha, headers)
            .await?;
        self.install_parsed_at::<M>(
            releases_base,
            owner,
            repository,
            &manifest,
            headers,
            upgrade,
        )
        .await
    }
}

/// Extract a downloaded release zip into `dir`. `None` bytes = the
/// manifest didn't declare this asset — no-op.
pub(crate) async fn write_zip_branch(
    dir: PathBuf,
    zip_bytes: Option<Vec<u8>>,
) -> Result<(), InstallError> {
    let Some(bytes) = zip_bytes else {
        return Ok(());
    };
    tokio::fs::create_dir_all(&dir).await.map_err(|e| {
        InstallError::ZipExtract(dir.clone(), e.to_string())
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
    .map_err(|e| InstallError::ZipExtract(dir.clone(), format!("join: {e}")))?
    .map_err(|e| InstallError::ZipExtract(dir.clone(), e))?;
    Ok(())
}

pub(crate) async fn write_manifest_branch(
    manifest_path: PathBuf,
    bytes: Vec<u8>,
) -> Result<(), InstallError> {
    // Atomic replace: the manifest is the installed-ness commit gate —
    // it must appear fully-written or not at all.
    crate::filesystem::util::write_atomic(&manifest_path, &bytes)
        .await
        .map_err(|e| {
            InstallError::ManifestPersist(manifest_path.clone(), e)
        })
}
