//! `update` — bare-naked streaming handler.
//!
//! Refreshes all five shipped binaries (cli, api, viewer, mcp, db)
//! from the latest GitHub release, emitting one [`ResponseItem`] per
//! (asset, stage) pair as the run progresses.
//!
//! Ported from `src/updater.rs`. The legacy emitted
//! `Notification::Updater` envelopes through a `cli::output::Handle`;
//! this version sends `ResponseItem` events through an
//! `mpsc::channel` whose receiver is surfaced as the leaf's
//! streaming return value.
//!
//! Layout on disk (resolved via `ctx.filesystem.bin_dir()` — every
//! binary is machine-wide, shared across states):
//!
//! ```text
//! <bin_dir>/objectiveai{.exe}        ← cli
//! <bin_dir>/objectiveai-api{.exe}
//! <bin_dir>/objectiveai-viewer{.exe}
//! <bin_dir>/objectiveai-mcp{.exe}
//! <bin_dir>/objectiveai-db{.exe}
//! ```
//!
//! Release-completeness rule: if the latest release is missing any of
//! the five expected `(os, arch)` assets, the run emits
//! [`ResponseSkipReason::IncompleteRelease`] and exits without
//! touching disk. Either all five advance together or none do.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use futures::Stream;
use objectiveai_sdk::cli::command::update::{Request, ResponseItem, ResponseSkipReason};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

const RELEASES_API: &str =
    "https://api.github.com/repos/ObjectiveAI/objectiveai/releases/latest";
const METADATA_TIMEOUT: Duration = Duration::from_secs(10);
// Per-asset download cap. Sized for the largest asset: `objectiveai-db`
// bundles postgres (~163 MB), so a tight cap would time out the db leg
// on slower links (the other binaries are far smaller).
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// Swap order: cli last because it's the running binary. The
/// sub-binaries don't gate each other — a swap failure on one is
/// non-fatal (we emit a per-binary error event and keep going).
const PACKAGES: &[&str] = &["api", "viewer", "mcp", "db", "cli"];

pub async fn execute(ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<ResponseItem, Error>>(8);
    let bin_dir = ctx.filesystem.bin_dir();
    // The GitHub credential lives in the on-disk json config only
    // (`api config github-authorization set`), not the env Config.
    let github_authorization = ctx
        .filesystem
        .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
        .await?
        .api()
        .get_github_authorization()
        .map(String::from);

    tokio::spawn(async move {
        if let Err(e) = run(&bin_dir, github_authorization.as_deref(), &tx).await {
            let _ = tx.send(Err(e)).await;
        }
    });

    Ok(Box::pin(futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    })))
}

async fn run(
    bin_dir: &Path,
    github_authorization: Option<&str>,
    tx: &tokio::sync::mpsc::Sender<Result<ResponseItem, Error>>,
) -> Result<(), Error> {
    // Dev tree refuses to self-update — overwriting a `cargo run`
    // output would clobber an in-tree build.
    let current_exe = std::env::current_exe()
        .map_err(|e| Error::Updater(format!("could not locate current binary: {e}")))?;
    if looks_like_dev_tree(&current_exe) {
        let _ = tx
            .send(Ok(ResponseItem::Skipped {
                reason: ResponseSkipReason::DevTree,
            }))
            .await;
        return Ok(());
    }

    let Some((os, arch, ext)) = platform_triple() else {
        let _ = tx
            .send(Ok(ResponseItem::Skipped {
                reason: ResponseSkipReason::UnsupportedPlatform,
            }))
            .await;
        return Ok(());
    };

    // Best-effort cleanup of any stale `.exe.old` from a prior Windows
    // swap before we begin.
    sweep_stale_old(&current_exe);

    // Build the five expected asset names for this (os, arch). The cli
    // is the bare `objectiveai-<os>-<arch>{ext}`; every other package
    // appends `-<package>` after the arch.
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
    let local_ver = semver::Version::parse(local)
        .map_err(|e| Error::Updater(format!("semver parse: {e}")))?;

    let _ = tx
        .send(Ok(ResponseItem::Checking {
            asset_name: format!("objectiveai-{os}-{arch}{ext}"),
            current_version: local.to_string(),
        }))
        .await;

    // Fetch the latest release metadata.
    let http = reqwest::Client::new();
    let auth = github_authorization_header(github_authorization);

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
            .map_err(|e| Error::Updater(format!("http: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::Updater(format!("github returned status {status}")));
        }
        let body = resp
            .bytes()
            .await
            .map_err(|e| Error::Updater(format!("http: {e}")))?;
        serde_json::from_slice(&body)
            .map_err(|e| Error::Updater(format!("malformed release metadata: {e}")))?
    };

    // Index assets by name; check all 5 expected names are present.
    let assets_map: std::collections::HashMap<&str, &Asset> = release
        .assets
        .iter()
        .map(|a| (a.name.as_str(), a))
        .collect();

    for (_, name) in &expected {
        if !assets_map.contains_key(name.as_str()) {
            let _ = tx
                .send(Ok(ResponseItem::Skipped {
                    reason: ResponseSkipReason::IncompleteRelease,
                }))
                .await;
            return Ok(());
        }
    }

    // Compare versions. Tag is `v<X.Y.Z>` by repo convention.
    let remote_str = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let remote = semver::Version::parse(remote_str)
        .map_err(|e| Error::Updater(format!("semver parse: {e}")))?;
    if remote <= local_ver {
        let _ = tx
            .send(Ok(ResponseItem::UpToDate {
                current_version: local_ver.to_string(),
                remote_version: remote.to_string(),
            }))
            .await;
        return Ok(());
    }

    std::fs::create_dir_all(bin_dir)
        .map_err(|e| Error::Updater(format!("create bin dir: {e}")))?;

    let targets: Vec<(&'static str, String, PathBuf)> = expected
        .iter()
        .map(|(pkg, name)| {
            let path = if *pkg == "cli" {
                bin_dir.join(format!("objectiveai{ext}"))
            } else {
                bin_dir.join(format!("objectiveai-{pkg}{ext}"))
            };
            (*pkg, name.clone(), path)
        })
        .collect();

    // Download all 5 to staged paths next to their targets.
    let pid = std::process::id();
    let mut staged: Vec<(&'static str, PathBuf, PathBuf)> = Vec::new();

    for (pkg, name, target) in &targets {
        let asset = assets_map
            .get(name.as_str())
            .expect("incomplete-release check above guarantees presence");
        let stage = staged_path(target, pid);

        let _ = tx
            .send(Ok(ResponseItem::Found {
                current_version: local_ver.to_string(),
                remote_version: remote.to_string(),
                asset_name: name.clone(),
                url: asset.browser_download_url.clone(),
            }))
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
                .map_err(|e| Error::Updater(format!("swap: {e}")))?;
        }

        staged.push((pkg, stage, target.clone()));
    }

    // Swap each staged binary into place. Order = PACKAGES order =
    // api/viewer/mcp first, cli last. Per-binary failures on
    // api/viewer/mcp emit a non-fatal Err event and continue; a cli
    // failure is fatal (returns Err).
    for (pkg, stage, target) in &staged {
        match self_replace(target, stage) {
            Ok(()) => {
                sweep_stale_old(target);
                let _ = tx
                    .send(Ok(ResponseItem::Installed {
                        current_version: local_ver.to_string(),
                        remote_version: remote.to_string(),
                    }))
                    .await;
            }
            Err(e) if *pkg == "cli" => return Err(e),
            Err(e) => {
                let _ = tx
                    .send(Err(Error::Updater(format!("{pkg}: swap failed: {e}"))))
                    .await;
            }
        }
    }

    Ok(())
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

fn github_authorization_header(caller: Option<&str>) -> Option<String> {
    caller.map(str::trim).filter(|s| !s.is_empty()).map(|s| {
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
        .map_err(|e| Error::Updater(format!("http: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::Updater(format!("github returned status {status}")));
    }

    let mut file = tokio::fs::File::create(dst)
        .await
        .map_err(|e| Error::Updater(format!("download: {e}")))?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| Error::Updater(format!("http: {e}")))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| Error::Updater(format!("download: {e}")))?;
    }
    file.flush()
        .await
        .map_err(|e| Error::Updater(format!("download: {e}")))?;
    Ok(())
}

/// Swap the staged binary into place.
///
/// **Unix**: `rename(new, current)` works because the running process
/// holds the binary by inode.
///
/// **Windows**: the running exe's path is locked for writes, but
/// renaming the file to a different name is allowed. Move current
/// aside, drop the new file into place. For fresh-install targets
/// (the binary didn't exist locally), skip the rename-aside step.
#[cfg(unix)]
fn self_replace(current: &Path, new: &Path) -> Result<(), Error> {
    std::fs::rename(new, current).map_err(|e| Error::Updater(format!("swap: {e}")))
}

#[cfg(windows)]
fn self_replace(current: &Path, new: &Path) -> Result<(), Error> {
    let old = current.with_extension("exe.old");
    let _ = std::fs::remove_file(&old);
    if current.exists() {
        std::fs::rename(current, &old).map_err(|e| Error::Updater(format!("swap: {e}")))?;
    }
    std::fs::rename(new, current).map_err(|e| {
        // Best effort: restore the original so the user isn't left
        // with a missing binary on PATH.
        let _ = std::fs::rename(&old, current);
        Error::Updater(format!("swap: {e}"))
    })
}

#[cfg(not(any(unix, windows)))]
fn self_replace(_current: &Path, _new: &Path) -> Result<(), Error> {
    Err(Error::Updater(
        "self-replace not implemented on this platform".to_string(),
    ))
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

pub mod request_schema {
    use objectiveai_sdk::cli::command::update as sdk;
    use objectiveai_sdk::cli::command::update::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::update as sdk;
    use objectiveai_sdk::cli::command::update::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
