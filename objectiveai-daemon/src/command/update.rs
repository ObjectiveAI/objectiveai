//! `update` — bare-naked streaming handler.
//!
//! Refreshes every shipped binary from the latest GitHub release by
//! downloading a single per-platform zip
//! (`objectiveai-<version>-<os>-<arch>.zip`) and replacing the contents
//! of the machine-wide `bin/` directory wholesale. Emits one
//! [`ResponseItem`] per stage as the run progresses.
//!
//! Layout on disk (resolved via `scoped.filesystem.bin_dir()` — every
//! binary is machine-wide, shared across states):
//!
//! ```text
//! <bin_dir>/objectiveai{.exe}                       ← cli
//! <bin_dir>/objectiveai-api{.exe}
//! <bin_dir>/objectiveai-viewer{.exe}
//! <bin_dir>/objectiveai-db{.exe}
//! <bin_dir>/objectiveai-claude-agent-sdk-runner{.exe}
//! <bin_dir>/objectiveai-codex-sdk-runner{.exe}
//! <bin_dir>/objectiveai-mcp-laboratory   (always musl-linux; no .exe)
//! ```
//!
//! Flow:
//! 1. Resolve the single zip asset for this `(os, arch)`. A missing zip
//!    emits [`ResponseSkipReason::IncompleteRelease`].
//! 2. Download the zip to a temp path (outside `bin/`, so the wipe
//!    below can't clobber it).
//! 3. Kill the running servers: this daemon's leashed resident
//!    children first, then a legacy lock-owner sweep (machine-wide
//!    `api` at `<bin_dir>/locks`; per-state `db` / `viewer`) for
//!    old-style detached servers left by ≤2.2.12 installs.
//! 4. Rename the running updater aside (Windows can't overwrite a
//!    running `.exe`; renaming frees the name — the process keeps
//!    running from the renamed file).
//! 5. Wipe `bin/` keeping only `plugins/` and `tools/` (best-effort —
//!    a still-running server/`.old` that won't delete is skipped).
//!    `pg-bin/` is wiped; postgres re-extracts on next `db` spawn.
//! 6. Unzip the download into `bin/`. Each binary is written via the
//!    same rename-aside swap so a still-locked straggler doesn't fail
//!    the extraction.
//!
//! Caveat: a server that comes back mid-install (or a binary still
//! locked at unzip time) is left as a `.old`; the cli's own `.old` is
//! swept on the next invocation.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use futures::Stream;
use objectiveai_sdk::cli::command::update::{Request, ResponseItem, ResponseSkipReason};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

const RELEASES_API: &str =
    "https://api.github.com/repos/ObjectiveAI/objectiveai/releases/latest";
const METADATA_TIMEOUT: Duration = Duration::from_secs(10);
// The per-platform zip bundles every binary (objectiveai-db alone
// carries postgres at ~163 MB), so the cap is generous to tolerate the
// full archive on slower links.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

/// Entries under `bin/` the wipe preserves. EMPTY since the installed
/// plugin/tool trees were retired (plugins are container images built
/// on laboratory hosts now) — the mechanism stays for whatever needs
/// preserving next.
const WIPE_KEEP: &[&str] = &[];

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<ItemStream, Error> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<ResponseItem, Error>>(8);
    let bin_dir = scoped.filesystem.bin_dir();
    // The GitHub credential lives in the on-disk json config only
    // (`api config github-authorization set`), not the env Config.
    let github_authorization = scoped
        .filesystem
        .read_config()
        .await?
        .api()
        .get_github_authorization()
        .map(String::from);

    let global = global.clone();
    tokio::spawn(async move {
        if let Err(e) = run(
            &global,
            &bin_dir,
            github_authorization.as_deref(),
            &tx,
        )
        .await
        {
            let _ = tx.send(Err(e)).await;
        }
    });

    Ok(Box::pin(futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    })))
}

async fn run(
    global: &GlobalContext,
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

    let local = env!("CARGO_PKG_VERSION");
    let local_ver = semver::Version::parse(local)
        .map_err(|e| Error::Updater(format!("semver parse: {e}")))?;

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

    // Compare versions. Tag is `v<X.Y.Z>` by repo convention. The asset
    // name embeds this version, so it has to be resolved before we can
    // look the asset up.
    let remote_str = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let remote = semver::Version::parse(remote_str)
        .map_err(|e| Error::Updater(format!("semver parse: {e}")))?;

    // One asset per platform, version-stamped:
    // objectiveai-<version>-<os>-<arch>.zip.
    let asset_name = format!("objectiveai-{remote_str}-{os}-{arch}.zip");

    let _ = tx
        .send(Ok(ResponseItem::Checking {
            asset_name: asset_name.clone(),
            current_version: local.to_string(),
        }))
        .await;

    // The platform's zip must be present, or there's nothing to install.
    let Some(asset) = release.assets.iter().find(|a| a.name == asset_name) else {
        let _ = tx
            .send(Ok(ResponseItem::Skipped {
                reason: ResponseSkipReason::IncompleteRelease,
            }))
            .await;
        return Ok(());
    };

    if remote <= local_ver {
        let _ = tx
            .send(Ok(ResponseItem::UpToDate {
                current_version: local_ver.to_string(),
                remote_version: remote.to_string(),
            }))
            .await;
        return Ok(());
    }

    let _ = tx
        .send(Ok(ResponseItem::Found {
            current_version: local_ver.to_string(),
            remote_version: remote.to_string(),
            asset_name: asset_name.clone(),
            url: asset.browser_download_url.clone(),
        }))
        .await;

    // Download the zip to a temp path OUTSIDE bin/ — the wipe below
    // clears bin/, so a staged copy in there would be deleted.
    let zip_path =
        std::env::temp_dir().join(format!("objectiveai-update-{}.zip", std::process::id()));
    if let Err(e) =
        download_to(&http, &asset.browser_download_url, auth.as_deref(), &zip_path, local).await
    {
        let _ = std::fs::remove_file(&zip_path);
        return Err(e);
    }

    // Kill the running servers before touching bin/: on Windows a live
    // child holds its .exe file-locked, which would defeat the wipe.
    // This daemon's leashed resident children — the laboratory host
    // GRACEFULLY (stdin EOF; it stops its containers first, and the
    // update waits that out, unbounded, BY DESIGN — no premature cap;
    // a wedged host blocks the update rather than leaking running
    // containers), the rest by signal.
    for key in ["api", "db", "viewer", "laboratories"] {
        kill_resident_child(global, key).await;
    }

    // Free the running updater's own slot so the unzip can replace it
    // (Windows can't overwrite a running .exe; renaming aside frees the
    // name). On Unix the wipe can unlink the running binary directly.
    rename_running_cli_aside(&current_exe);

    // Wipe bin/ except the preserved entries, then lay down the new set.
    wipe_bin_except(bin_dir, WIPE_KEEP);
    std::fs::create_dir_all(bin_dir)
        .map_err(|e| Error::Updater(format!("create bin dir: {e}")))?;

    let unzip_result = unzip_into(&zip_path, bin_dir);
    let _ = std::fs::remove_file(&zip_path);
    unzip_result?;

    sweep_stale_old(&current_exe);

    let _ = tx
        .send(Ok(ResponseItem::Installed {
            current_version: local_ver.to_string(),
            remote_version: remote.to_string(),
        }))
        .await;

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
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        Some(("windows", "aarch64", ".exe"))
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
    )))]
    {
        None
    }
}

fn looks_like_dev_tree(current_exe: &Path) -> bool {
    current_exe.components().any(|c| {
        let s = c.as_os_str();
        s == "target"
            || s == "target-objectiveai-mcp-laboratory"
            || s == "target-objectiveai-mcp-proxy"
            || s == "target-objectiveai-viewer"
    })
}

use crate::command::kill_helpers::kill_resident_child;

#[cfg(windows)]
fn rename_running_cli_aside(current_exe: &Path) {
    // Only matters when the running binary lives in the directory the
    // unzip will overwrite; renaming it frees its name.
    let old = current_exe.with_extension("exe.old");
    let _ = std::fs::remove_file(&old);
    let _ = std::fs::rename(current_exe, &old);
}

#[cfg(not(windows))]
fn rename_running_cli_aside(_current_exe: &Path) {
    // Unix can unlink a running binary directly (the inode survives), so
    // the wipe + unzip handle it with no rename needed.
}

/// Delete every entry under `bin_dir` except `keep`. Best-effort: a
/// still-running server binary (or the renamed-aside updater) that
/// won't delete is silently skipped — the unzip's swap handles it.
fn wipe_bin_except(bin_dir: &Path, keep: &[&str]) {
    let rd = match std::fs::read_dir(bin_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in rd.flatten() {
        let name = entry.file_name();
        if keep.iter().any(|k| std::ffi::OsStr::new(k) == name) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Extract every file in `zip_path` into `bin_dir` (flattened to the
/// archive's bare filenames). Each file is written to a staged
/// `.new.<pid>` sibling, made executable on Unix, then swapped into
/// place — so a target still locked by a running process is moved aside
/// rather than failing the write.
fn unzip_into(zip_path: &Path, bin_dir: &Path) -> Result<(), Error> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| Error::Updater(format!("open zip: {e}")))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Error::Updater(format!("read zip: {e}")))?;
    let pid = std::process::id();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| Error::Updater(format!("read zip entry: {e}")))?;
        if !entry.is_file() {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let Some(file_name) = rel.file_name() else {
            continue;
        };
        let dst = bin_dir.join(file_name);
        let staged = staged_path(&dst, pid);

        {
            let mut out = std::fs::File::create(&staged)
                .map_err(|e| Error::Updater(format!("unzip: {e}")))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| Error::Updater(format!("unzip: {e}")))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| Error::Updater(format!("unzip chmod: {e}")))?;
        }

        if let Err(e) = self_replace(&dst, &staged) {
            let _ = std::fs::remove_file(&staged);
            return Err(e);
        }
    }

    Ok(())
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
/// **Unix**: `rename(new, current)` works because a running process
/// holds its binary by inode.
///
/// **Windows**: the running exe's path is locked for writes, but
/// renaming the file to a different name is allowed. Move current aside,
/// drop the new file into place. For fresh targets (the binary didn't
/// exist — e.g. just wiped), skip the rename-aside step.
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

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::update as sdk;
    use objectiveai_sdk::cli::command::update::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
