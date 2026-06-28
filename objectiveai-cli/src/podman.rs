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
//! (`<bin>/locks`, key `podman`) and gated by a `.objectiveai-complete`
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
    let marker = root.join(".objectiveai-complete");
    let exe = root.join(&target.exe_rel);

    // 1. Fast path: a completed install — no lock.
    if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
        return Ok(exe);
    }

    // 2. Serialize installs machine-wide (a sibling may be mid-extract;
    //    we wait rather than race).
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &bin_dir.join("locks"),
        "podman",
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| Error::Podman(format!("bin lock: {e}")))?;

    // 3. Install under the claim, then release explicitly — dropping a
    //    LockClaim deliberately does NOT release it (mirrors
    //    `python::initialize`).
    let result = install_under_lock(&bin_dir, &root, &marker, &exe, &target).await;
    claim
        .release()
        .map_err(|e| Error::Podman(format!("bin lock release: {e}")))?;
    result?;
    Ok(exe)
}

/// Download + extract under the bin lock. Caller holds the claim.
async fn install_under_lock(
    bin_dir: &Path,
    root: &Path,
    marker: &Path,
    exe: &Path,
    target: &Target,
) -> Result<(), Error> {
    // Re-probe under the lock — a sibling may have finished while we waited.
    if tokio::fs::try_exists(marker).await.unwrap_or(false) {
        return Ok(());
    }

    let podman_dir = bin_dir.join("podman");

    // A partial extract has the dir but no marker. Move it aside before
    // re-extracting — rename-then-delete, not delete-in-place: Windows
    // directory deletion is asynchronous, so a plain `remove_dir_all`
    // returns while the tree is still pending-delete and the immediate
    // re-create races ACCESS_DENIED. The rename frees the name instantly;
    // deleting the renamed tree is best-effort.
    if tokio::fs::try_exists(root).await.unwrap_or(false) {
        let trash = podman_dir.join(format!("{}.trash-{}", target.version, std::process::id()));
        tokio::fs::rename(root, &trash)
            .await
            .map_err(|e| Error::Podman(format!("move partial install aside: {e}")))?;
        let _ = tokio::fs::remove_dir_all(&trash).await;
    }
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
    let download = download_to(&target.url, &archive).await;
    if let Err(e) = download {
        let _ = tokio::fs::remove_file(&archive).await;
        return Err(e);
    }

    // Extract (synchronous crates) on a blocking thread.
    let extract = {
        let archive = archive.clone();
        let root = root.to_path_buf();
        let kind = match target.kind {
            ArchiveKind::Zip => ArchiveKind::Zip,
            ArchiveKind::TarGz => ArchiveKind::TarGz,
        };
        tokio::task::spawn_blocking(move || extract_archive(&archive, &root, kind))
            .await
            .map_err(|e| Error::Podman(format!("extract task join: {e}")))?
    };
    let _ = tokio::fs::remove_file(&archive).await;
    extract?;

    // Guard against upstream layout drift: the exe must be where we expect.
    if !tokio::fs::try_exists(exe).await.unwrap_or(false) {
        return Err(Error::Podman(format!(
            "podman executable not found after extract at {exe:?}"
        )));
    }

    // Marker last — its presence means a COMPLETE install.
    tokio::fs::write(marker, b"")
        .await
        .map_err(|e| Error::Podman(format!("write {marker:?}: {e}")))?;
    Ok(())
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
