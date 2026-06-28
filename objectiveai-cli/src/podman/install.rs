//! Runtime podman installer.
//!
//! [`ensure_installed`] lazily downloads + installs podman into
//! `<bin>/podman/<version>/` on first use and returns the path to the
//! podman executable; it is driven (and memoized) by
//! [`crate::context::Context::podman`].
//!
//! What gets installed is platform-specific, because "podman" is binned
//! differently per OS:
//! - **Linux** — the full static rootless engine bundle from
//!   `mgoltzsche/podman-static` (podman + crun/runc + conmon + netavark +
//!   aardvark-dns + pasta + fuse-overlayfs + config), a `.tar.gz` tree.
//!   Runs containers natively on the host — no VM.
//! - **macOS / Windows** — the official `containers/podman` remote-client
//!   release zip. The container engine itself lives in a `podman machine`
//!   (a VM on macOS, WSL2 on Windows) that podman downloads/sets up on
//!   demand — none of that is bundled here.
//!
//! Concurrency mirrors [`crate::python`] / `objectiveai-db`'s installer:
//! in-process callers coalesce on the `Context`'s `OnceCell`; across
//! processes the install is serialized by the bin lock
//! (`<bin>/locks`, key `podman`) and gated by a `.objectiveai-install-complete`
//! marker (probe → `wait_acquire` → re-probe → install → marker →
//! explicit release). A partial install (dir present, marker absent) is
//! renamed aside and redone.

use std::path::{Path, PathBuf};

use crate::error::Error;

// One pinned version per (os, arch) so any single platform can be bumped
// independently. All identical for now — every source hosts v5.8.4.
pub const PODMAN_VERSION_LINUX_AMD64: &str = "5.8.4"; // mgoltzsche/podman-static
pub const PODMAN_VERSION_LINUX_ARM64: &str = "5.8.4"; // mgoltzsche/podman-static
pub const PODMAN_VERSION_MACOS_AMD64: &str = "5.8.4"; // containers/podman
pub const PODMAN_VERSION_MACOS_ARM64: &str = "5.8.4"; // containers/podman
pub const PODMAN_VERSION_WINDOWS_AMD64: &str = "5.8.4"; // containers/podman
pub const PODMAN_VERSION_WINDOWS_ARM64: &str = "5.8.4"; // containers/podman

/// Download timeout for the (tens-of-MB) podman archive.
const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

#[derive(Clone, Copy)]
enum ArchiveKind {
    Zip,
    TarGz,
}

/// The resolved per-`(os, arch)` install target.
struct Target {
    version: &'static str,
    url: String,
    kind: ArchiveKind,
    /// Relative path of the podman executable inside the extracted tree.
    exe_rel: PathBuf,
    /// `.zip` or `.tar.gz` — used to name the temp download.
    ext: &'static str,
}

/// Resolve the download URL + archive layout for the host `(os, arch)`.
///
/// The mapping (kept here as the single source of truth):
/// - `linux`   → mgoltzsche static engine tarball; exe at
///   `podman-linux-<arch>/usr/local/bin/podman`.
/// - `macos`   → official darwin remote zip; exe at
///   `podman-<version>/usr/bin/podman`.
/// - `windows` → official windows remote zip; exe at
///   `podman-<version>/usr/bin/podman.exe`.
fn resolve_target() -> Result<Target, Error> {
    // podman/mgoltzsche name arches amd64/arm64.
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => {
            return Err(Error::Podman(format!("unsupported architecture: {other}")));
        }
    };
    match std::env::consts::OS {
        "linux" => {
            let version = if arch == "amd64" {
                PODMAN_VERSION_LINUX_AMD64
            } else {
                PODMAN_VERSION_LINUX_ARM64
            };
            Ok(Target {
                version,
                url: format!(
                    "https://github.com/mgoltzsche/podman-static/releases/download/v{version}/podman-linux-{arch}.tar.gz"
                ),
                kind: ArchiveKind::TarGz,
                exe_rel: PathBuf::from(format!("podman-linux-{arch}/usr/local/bin/podman")),
                ext: "tar.gz",
            })
        }
        "macos" => {
            let version = if arch == "amd64" {
                PODMAN_VERSION_MACOS_AMD64
            } else {
                PODMAN_VERSION_MACOS_ARM64
            };
            Ok(Target {
                url: format!(
                    "https://github.com/containers/podman/releases/download/v{version}/podman-remote-release-darwin_{arch}.zip"
                ),
                kind: ArchiveKind::Zip,
                exe_rel: PathBuf::from(format!("podman-{version}/usr/bin/podman")),
                ext: "zip",
                version,
            })
        }
        "windows" => {
            let version = if arch == "amd64" {
                PODMAN_VERSION_WINDOWS_AMD64
            } else {
                PODMAN_VERSION_WINDOWS_ARM64
            };
            Ok(Target {
                url: format!(
                    "https://github.com/containers/podman/releases/download/v{version}/podman-remote-release-windows_{arch}.zip"
                ),
                kind: ArchiveKind::Zip,
                exe_rel: PathBuf::from(format!("podman-{version}/usr/bin/podman.exe")),
                ext: "zip",
                version,
            })
        }
        other => Err(Error::Podman(format!("unsupported OS: {other}"))),
    }
}

/// Ensure podman is installed under `<bin_dir>/podman/<version>/` and
/// return the path to the podman executable, installing it once if
/// needed. Concurrency-safe across in-process callers (via the caller's
/// `OnceCell`) and across processes (via the bin lock + completion
/// marker). See the module docs.
pub async fn ensure_installed(bin_dir: PathBuf) -> Result<PathBuf, Error> {
    let target = resolve_target()?;
    let root = bin_dir.join("podman").join(target.version);
    let marker = root.join(".objectiveai-install-complete");
    let exe = root.join(&target.exe_rel);

    // 1. Fast path: a completed install — no lock.
    if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
        return Ok(exe);
    }

    // 2. Serialize installs machine-wide. We may BLOCK here while a sibling
    //    process is mid-install (or about to finish one).
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &bin_dir.join("locks"),
        "podman",
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| Error::Podman(format!("bin lock: {e}")))?;

    // 3. DOUBLE-CHECKED LOCKING: re-check the marker now that we hold the
    //    lock — a sibling may have completed the install while we were
    //    blocked acquiring it. Only install if it is STILL missing. The
    //    install (download + extract + marker) runs entirely under the lock.
    let result = async {
        if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
            return Ok(());
        }
        install(&bin_dir, &root, &marker, &exe, &target).await
    }
    .await;

    // 4. Release explicitly on EVERY path — dropping a LockClaim
    //    deliberately does NOT release it (mirrors `python::initialize`).
    //    Release before propagating an install error so the lock is never
    //    leaked.
    claim
        .release()
        .map_err(|e| Error::Podman(format!("bin lock release: {e}")))?;
    result?;
    Ok(exe)
}

/// Download + extract into `root`. The caller holds the bin lock and has
/// already re-checked (under the lock) that the install is missing.
async fn install(
    bin_dir: &Path,
    root: &Path,
    marker: &Path,
    exe: &Path,
    target: &Target,
) -> Result<(), Error> {
    let podman_dir = bin_dir.join("podman");

    // A partial extract has the dir but no marker. Move it aside before
    // re-extracting — rename-then-delete, not delete-in-place: Windows
    // directory deletion is asynchronous, so a plain `remove_dir_all`
    // returns while the tree is still pending-delete and the immediate
    // re-create races ACCESS_DENIED. The rename frees the name instantly.
    if tokio::fs::try_exists(root).await.unwrap_or(false) {
        let trash = podman_dir.join(format!("{}.trash-{}", target.version, std::process::id()));
        tokio::fs::rename(root, &trash)
            .await
            .map_err(|e| Error::Podman(format!("move partial install aside: {e}")))?;
    }
    // Best-effort sweep of leftover trash dirs / download temp files (this
    // run's renamed-aside tree, plus anything a crashed prior run left).
    // Safe — we hold the lock; prevents unbounded accumulation.
    sweep_leftovers(&podman_dir).await;

    tokio::fs::create_dir_all(root)
        .await
        .map_err(|e| Error::Podman(format!("mkdir {root:?}: {e}")))?;

    // Download the archive to a same-filesystem temp beside the install.
    let archive = podman_dir.join(format!(
        "{}.download-{}.{}",
        target.version,
        std::process::id(),
        target.ext
    ));
    if let Err(e) = download_to(&target.url, &archive).await {
        let _ = tokio::fs::remove_file(&archive).await;
        return Err(e);
    }

    // Extract, retrying a couple of times: transient Windows AV interference
    // can briefly pin a just-written executable (mirrors objectiveai-db).
    // Both extractors overwrite, so a partial tree is fine to extract over —
    // no wipe between tries (which would reintroduce the async-delete race).
    let mut result = run_extract(&archive, root, target.kind).await;
    for _ in 0..2 {
        if result.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        result = run_extract(&archive, root, target.kind).await;
    }
    let _ = tokio::fs::remove_file(&archive).await;
    result?;

    // Guard against upstream layout drift: the exe must be where we expect.
    if !tokio::fs::try_exists(exe).await.unwrap_or(false) {
        return Err(Error::Podman(format!(
            "podman executable not found after extract at {exe:?}"
        )));
    }

    // Marker LAST — its presence is the signal that the install is COMPLETE.
    tokio::fs::write(marker, b"")
        .await
        .map_err(|e| Error::Podman(format!("write {marker:?}: {e}")))?;
    Ok(())
}

/// Run the synchronous [`extract_archive`] on a blocking thread.
async fn run_extract(archive: &Path, root: &Path, kind: ArchiveKind) -> Result<(), Error> {
    let archive = archive.to_path_buf();
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || extract_archive(&archive, &root, kind))
        .await
        .map_err(|e| Error::Podman(format!("extract task join: {e}")))?
}

/// Best-effort removal of `*.trash-*` dirs and `*.download-*` temp files
/// under the podman dir — leftovers from this run's rename-aside or a
/// crashed prior install. Caller holds the lock, so this is race-free.
async fn sweep_leftovers(podman_dir: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(podman_dir).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.contains(".trash-") && !name.contains(".download-") {
            continue;
        }
        let path = entry.path();
        if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            let _ = tokio::fs::remove_dir_all(&path).await;
        } else {
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}

/// Extract `archive` into `dest`, preserving unix mode bits (so the
/// executables stay executable). Synchronous — run on a blocking thread.
fn extract_archive(archive: &Path, dest: &Path, kind: ArchiveKind) -> Result<(), Error> {
    let file = std::fs::File::open(archive)
        .map_err(|e| Error::Podman(format!("open {archive:?}: {e}")))?;
    match kind {
        ArchiveKind::Zip => {
            let mut zip = zip::ZipArchive::new(file)
                .map_err(|e| Error::Podman(format!("read zip: {e}")))?;
            zip.extract(dest)
                .map_err(|e| Error::Podman(format!("extract zip: {e}")))?;
        }
        ArchiveKind::TarGz => {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut tar = tar::Archive::new(decoder);
            tar.unpack(dest)
                .map_err(|e| Error::Podman(format!("extract tar.gz: {e}")))?;
        }
    }
    Ok(())
}

/// Stream an HTTP GET to `dst`. Adapted from
/// `crate::command::update::download_to`.
async fn download_to(url: &str, dst: &Path) -> Result<(), Error> {
    use futures::StreamExt as _;
    use tokio::io::AsyncWriteExt as _;

    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header(
            "User-Agent",
            format!("objectiveai/{}", env!("CARGO_PKG_VERSION")),
        )
        .timeout(DOWNLOAD_TIMEOUT)
        .send()
        .await
        .map_err(|e| Error::Podman(format!("http: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::Podman(format!("download {url}: status {status}")));
    }

    let mut file = tokio::fs::File::create(dst)
        .await
        .map_err(|e| Error::Podman(format!("create {dst:?}: {e}")))?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| Error::Podman(format!("http: {e}")))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| Error::Podman(format!("write {dst:?}: {e}")))?;
    }
    file.flush()
        .await
        .map_err(|e| Error::Podman(format!("flush {dst:?}: {e}")))?;
    Ok(())
}
