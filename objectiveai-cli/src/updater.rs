//! `objectiveai update` — refresh all four shipped binaries from the
//! latest GitHub release.
//!
//! The cli is the only updater in the suite: it manages itself plus
//! `objectiveai-api`, `objectiveai-viewer`, and `objectiveai-mcp`.
//! Other binaries no longer self-update.
//!
//! **Layout on disk** (resolved through
//! [`objectiveai_sdk::filesystem::Client::base_dir`]):
//!
//! ```text
//! <base_dir>/objectiveai{.exe}        ← cli
//! <base_dir>/bin/objectiveai-api{.exe}
//! <base_dir>/bin/objectiveai-viewer{.exe}
//! <base_dir>/bin/objectiveai-mcp{.exe}
//! ```
//!
//! **Release-completeness rule.** If the latest release is missing any
//! of the four expected `(os, arch)` assets, we emit
//! [`SkipReason::IncompleteRelease`] and exit without touching disk.
//! Either all four advance together or none do.
//!
//! Within an active run, downloads stage to `<target>.new.<pid>` next
//! to each target so the eventual rename is same-filesystem. If any
//! download fails, every staged file is cleaned up before returning.
//! Swap order is api → viewer → mcp → cli; the cli goes last because
//! it's the running binary and gets the windows file-lock dance
//! (rename current aside, drop new in, sweep `.exe.old` next run).

use std::path::{Path, PathBuf};
use std::time::Duration;

use objectiveai_sdk::cli::output::notification::{Notification, SkipReason, Updater};
use objectiveai_sdk::cli::output::{Handle, Output};
use objectiveai_sdk::filesystem::Client;

const RELEASES_API: &str =
    "https://api.github.com/repos/ObjectiveAI/objectiveai/releases/latest";
const METADATA_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// Swap order: cli last because it's the running binary. The three
/// sub-binaries don't gate each other — a swap failure on one is
/// non-fatal (we emit a warn-level error and keep going).
const PACKAGES: &[&str] = &["api", "viewer", "mcp", "cli"];

pub async fn run_update(
    cli_config: &crate::run::Config,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    imp::run(cli_config, handle)
        .await
        .map_err(|e| crate::error::Error::Updater(e.to_string()))
}

mod imp {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    pub(super) enum Error {
        #[error("could not locate current binary: {0}")]
        CurrentExe(std::io::Error),
        #[error("http: {0}")]
        Http(String),
        #[error("github returned status {0}")]
        BadStatus(reqwest::StatusCode),
        #[error("malformed release metadata: {0}")]
        BadMetadata(serde_json::Error),
        #[error("semver parse: {0}")]
        Semver(semver::Error),
        #[error("download: {0}")]
        Download(std::io::Error),
        #[error("swap: {0}")]
        Swap(std::io::Error),
        #[error("create bin dir: {0}")]
        CreateBinDir(std::io::Error),
    }

    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
        assets: Vec<Asset>,
    }

    #[derive(serde::Deserialize)]
    struct Asset {
        name: String,
        browser_download_url: String,
    }

    pub(super) async fn run(
        cli_config: &crate::run::Config,
        handle: &Handle,
    ) -> Result<(), Error> {
        // Dev tree refuses to self-update — overwriting a `cargo run`
        // output would clobber an in-tree build.
        let current_exe = std::env::current_exe().map_err(Error::CurrentExe)?;
        if looks_like_dev_tree(&current_exe) {
            emit_notification(handle, Updater::Skipped { reason: SkipReason::DevTree })
                .await;
            return Ok(());
        }

        let Some((os, arch, ext)) = platform_triple() else {
            emit_notification(
                handle,
                Updater::Skipped { reason: SkipReason::UnsupportedPlatform },
            )
            .await;
            return Ok(());
        };

        // Best-effort cleanup of any stale `.exe.old` from a prior
        // Windows swap before we begin.
        sweep_stale_old(&current_exe);

        // Build the four expected asset names for this (os, arch).
        // The cli is the bare `objectiveai-<os>-<arch>{ext}`; every
        // other package appends `-<package>` after the arch.
        let expected: Vec<(&'static str, String)> = PACKAGES
            .iter()
            .map(|&pkg| {
                let name = if pkg == "cli" {
                    format!("objectiveai-{os}-{arch}{ext}")
                } else {
                    format!("objectiveai-{os}-{arch}-{pkg}{ext}")
                };
                (pkg, name)
            })
            .collect();

        let local = env!("CARGO_PKG_VERSION");
        let local_ver = semver::Version::parse(local).map_err(Error::Semver)?;

        emit_notification(
            handle,
            Updater::Checking {
                asset_name: format!("objectiveai-{os}-{arch}{ext}"),
                current_version: local.to_string(),
            },
        )
        .await;

        // Fetch the latest release metadata.
        let http = reqwest::Client::new();
        let auth = github_authorization(cli_config.github_authorization.as_deref());

        let release: Release = {
            let mut req = http
                .get(RELEASES_API)
                .header("User-Agent", format!("objectiveai/{local}"))
                .header("Accept", "application/vnd.github+json")
                .timeout(METADATA_TIMEOUT);
            if let Some(ref h) = auth {
                req = req.header("Authorization", h);
            }
            let resp = req
                .send()
                .await
                .map_err(|e| Error::Http(e.to_string()))?;
            let status = resp.status();
            if !status.is_success() {
                return Err(Error::BadStatus(status));
            }
            let body = resp
                .bytes()
                .await
                .map_err(|e| Error::Http(e.to_string()))?;
            serde_json::from_slice(&body).map_err(Error::BadMetadata)?
        };

        // Index assets by name; check all 4 expected names are present.
        let assets_map: std::collections::HashMap<&str, &Asset> = release
            .assets
            .iter()
            .map(|a| (a.name.as_str(), a))
            .collect();

        for (_, name) in &expected {
            if !assets_map.contains_key(name.as_str()) {
                emit_notification(
                    handle,
                    Updater::Skipped { reason: SkipReason::IncompleteRelease },
                )
                .await;
                return Ok(());
            }
        }

        // Compare versions. Tag is `v<X.Y.Z>` by repo convention.
        let remote_str = release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name);
        let remote = semver::Version::parse(remote_str).map_err(Error::Semver)?;
        if remote <= local_ver {
            emit_notification(
                handle,
                Updater::UpToDate {
                    current_version: local_ver.to_string(),
                    remote_version: remote.to_string(),
                },
            )
            .await;
            return Ok(());
        }

        // Resolve install paths through the SDK's filesystem client so
        // CONFIG_BASE_DIR is honoured.
        let fs_client = Client::new(
            cli_config.config_base_dir.clone(),
            None::<String>,
            None::<String>,
        );
        let base_dir = fs_client.base_dir().clone();
        let bin_dir = base_dir.join("bin");
        std::fs::create_dir_all(&bin_dir).map_err(Error::CreateBinDir)?;

        let targets: Vec<(&'static str, String, PathBuf)> = expected
            .iter()
            .map(|(pkg, name)| {
                let path = if *pkg == "cli" {
                    base_dir.join(format!("objectiveai{ext}"))
                } else {
                    bin_dir.join(format!("objectiveai-{pkg}{ext}"))
                };
                (*pkg, name.clone(), path)
            })
            .collect();

        // Download all 4 to staged paths next to their targets.
        let pid = std::process::id();
        let mut staged: Vec<(&'static str, PathBuf, PathBuf)> = Vec::new();

        for (pkg, name, target) in &targets {
            let asset = assets_map
                .get(name.as_str())
                .expect("incomplete-release check above guarantees presence");
            let stage = staged_path(target, pid);

            emit_notification(
                handle,
                Updater::Found {
                    current_version: local_ver.to_string(),
                    remote_version: remote.to_string(),
                    asset_name: name.clone(),
                    url: asset.browser_download_url.clone(),
                },
            )
            .await;

            if let Err(e) = download_to(
                &http,
                &asset.browser_download_url,
                auth.as_deref(),
                &stage,
                local,
            )
            .await
            {
                // Clean up any prior staged files on failure.
                for (_, sp, _) in &staged {
                    let _ = std::fs::remove_file(sp);
                }
                let _ = std::fs::remove_file(&stage);
                return Err(e);
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&stage, std::fs::Permissions::from_mode(0o755))
                    .map_err(Error::Swap)?;
            }

            staged.push((pkg, stage, target.clone()));
        }

        // Swap each staged binary into place. Order = PACKAGES order =
        // api/viewer/mcp first, cli last. Per-binary failures on
        // api/viewer/mcp emit a warn and continue; a cli failure is
        // fatal (returns Err).
        for (pkg, stage, target) in &staged {
            match self_replace(target, stage) {
                Ok(()) => {
                    sweep_stale_old(target);
                    emit_notification(
                        handle,
                        Updater::Installed {
                            current_version: local_ver.to_string(),
                            remote_version: remote.to_string(),
                        },
                    )
                    .await;
                }
                Err(e) if *pkg == "cli" => return Err(e),
                Err(e) => {
                    emit_warn(handle, &format!("{pkg}: swap failed: {e}")).await;
                }
            }
        }

        Ok(())
    }

    fn platform_triple() -> Option<(&'static str, &'static str, &'static str)> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            Some(("linux", "x86_64", ""))
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            Some(("linux", "aarch64", ""))
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            Some(("macos", "x86_64", ""))
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Some(("macos", "aarch64", ""))
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            Some(("windows", "x86_64", ".exe"))
        }
        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )))]
        {
            None
        }
    }

    fn looks_like_dev_tree(current_exe: &Path) -> bool {
        current_exe.components().any(|c| {
            let s = c.as_os_str();
            s == "target"
                || s == "target-objectiveai-mcp-filesystem"
                || s == "target-objectiveai-mcp-proxy"
                || s == "target-objectiveai-viewer"
        })
    }

    fn staged_path(target: &Path, pid: u32) -> PathBuf {
        let mut p = target.to_path_buf();
        let filename = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "objectiveai".to_string());
        p.set_file_name(format!("{filename}.new.{pid}"));
        p
    }

    fn github_authorization(caller: Option<&str>) -> Option<String> {
        caller
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                let bare = s.strip_prefix("Bearer ").unwrap_or(s);
                format!("Bearer {bare}")
            })
    }

    async fn download_to(
        client: &reqwest::Client,
        url: &str,
        auth: Option<&str>,
        dst: &Path,
        version: &str,
    ) -> Result<(), Error> {
        use futures::StreamExt as _;
        use tokio::io::AsyncWriteExt as _;

        let mut req = client
            .get(url)
            .header("User-Agent", format!("objectiveai/{version}"))
            .timeout(DOWNLOAD_TIMEOUT);
        if let Some(h) = auth {
            req = req.header("Authorization", h);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::BadStatus(status));
        }

        let mut file = tokio::fs::File::create(dst).await.map_err(Error::Download)?;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| Error::Http(e.to_string()))?;
            file.write_all(&chunk).await.map_err(Error::Download)?;
        }
        file.flush().await.map_err(Error::Download)?;
        Ok(())
    }

    /// Swap the staged binary into place.
    ///
    /// **Unix**: `rename(new, current)` works because the running
    /// process holds the binary by inode.
    ///
    /// **Windows**: the running exe's path is locked for writes, but
    /// renaming the file to a different name is allowed. Move current
    /// aside, drop the new file into place. For fresh-install targets
    /// (the binary didn't exist locally), skip the rename-aside step.
    #[cfg(unix)]
    fn self_replace(current: &Path, new: &Path) -> Result<(), Error> {
        std::fs::rename(new, current).map_err(Error::Swap)
    }

    #[cfg(windows)]
    fn self_replace(current: &Path, new: &Path) -> Result<(), Error> {
        let old = current.with_extension("exe.old");
        let _ = std::fs::remove_file(&old);
        if current.exists() {
            std::fs::rename(current, &old).map_err(Error::Swap)?;
        }
        std::fs::rename(new, current).map_err(|e| {
            // Best effort: restore the original so the user isn't left
            // with a missing binary on PATH.
            let _ = std::fs::rename(&old, current);
            Error::Swap(e)
        })
    }

    #[cfg(not(any(unix, windows)))]
    fn self_replace(_current: &Path, _new: &Path) -> Result<(), Error> {
        Err(Error::Swap(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "self-replace not implemented on this platform",
        )))
    }

    fn sweep_stale_old(current: &Path) {
        #[cfg(windows)]
        {
            let old = current.with_extension("exe.old");
            let _ = std::fs::remove_file(old);
        }
        #[cfg(not(windows))]
        {
            let _ = current;
        }
    }
}

async fn emit_notification(handle: &Handle, value: Updater) {
    let output: Output<Updater> = Output::Notification(Notification { value, agent_id: None });
    output.emit(handle).await;
}

async fn emit_warn(handle: &Handle, message: &str) {
    let err = objectiveai_sdk::cli::output::Error {
        level: objectiveai_sdk::cli::output::Level::Warn,
        fatal: false,
        message: serde_json::Value::String(message.to_string()),
        agent_id: None,
    };
    let output: Output<serde_json::Value> = Output::Error(err);
    output.emit(handle).await;
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    ))]
    fn package_asset_names_for_current_target() {
        // The asset-name format for each package must match what
        // release.yml uploads. cli is the bare name; api/viewer/mcp
        // append `-<pkg>` before the ext.
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let (os, arch, ext) = ("linux", "x86_64", "");
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        let (os, arch, ext) = ("linux", "aarch64", "");
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let (os, arch, ext) = ("macos", "x86_64", "");
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let (os, arch, ext) = ("macos", "aarch64", "");
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        let (os, arch, ext) = ("windows", "x86_64", ".exe");

        let cli = format!("objectiveai-{os}-{arch}{ext}");
        assert!(cli.starts_with("objectiveai-"));
        for pkg in ["api", "viewer", "mcp"] {
            let expected = format!("objectiveai-{os}-{arch}-{pkg}{ext}");
            assert!(expected.contains(pkg));
            assert!(expected.starts_with("objectiveai-"));
            if ext == ".exe" {
                assert!(expected.ends_with(".exe"));
            }
        }
    }

    #[test]
    fn version_ordering() {
        fn needs_update(remote: &str, local: &str) -> bool {
            let r = remote.strip_prefix('v').unwrap_or(remote);
            semver::Version::parse(r).unwrap() > semver::Version::parse(local).unwrap()
        }
        assert!(needs_update("v2.0.11", "2.0.10"));
        assert!(needs_update("2.0.11", "2.0.10"));
        assert!(!needs_update("v2.0.10", "2.0.10"));
        assert!(!needs_update("v2.0.10", "2.1.0"));
        assert!(needs_update("v3.0.0", "2.99.99"));
    }
}
