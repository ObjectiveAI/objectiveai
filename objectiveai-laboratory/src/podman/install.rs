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
//!   demand — that image is not bundled here.
//!   - **Windows** carries its machine helpers in the zip already
//!     (`gvproxy.exe`, `win-sshproxy.exe`).
//!   - **macOS** does NOT: the zip is just `podman` + `podman-mac-helper`, so
//!     after extracting we also fetch `vfkit` (the applehv VM monitor, with the
//!     virtualization entitlement) and `gvproxy` (networking) beside `podman`
//!     — podman won't auto-download those. See [`fetch_macos_helpers`].
//!
//! Concurrency mirrors `objectiveai-cli`'s python installer / `objectiveai-db`'s installer:
//! in-process callers coalesce on the `Context`'s `OnceCell`; across
//! processes the install is serialized by the bin lock
//! (`<bin>/locks`, key `podman`) and gated by a `.objectiveai-install-complete`
//! marker (probe → `wait_acquire` → re-probe → install → marker →
//! explicit release). A partial install (dir present, marker absent) is
//! renamed aside and redone.

use std::path::{Path, PathBuf};

use super::Error;

// One pinned version per (os, arch) so any single platform can be bumped
// independently. All identical for now — every source hosts v5.8.4.
pub const PODMAN_VERSION_LINUX_AMD64: &str = "5.8.4"; // mgoltzsche/podman-static
pub const PODMAN_VERSION_LINUX_ARM64: &str = "5.8.4"; // mgoltzsche/podman-static
pub const PODMAN_VERSION_MACOS_AMD64: &str = "5.8.4"; // containers/podman
pub const PODMAN_VERSION_MACOS_ARM64: &str = "5.8.4"; // containers/podman
pub const PODMAN_VERSION_WINDOWS_AMD64: &str = "5.8.4"; // containers/podman
pub const PODMAN_VERSION_WINDOWS_ARM64: &str = "5.8.4"; // containers/podman

// macOS `podman machine` (applehv) helpers that the remote-client zip omits.
// Pinned to the versions podman 5.8.4's official .pkg bundles (its
// contrib/pkginstaller: VFKIT_VERSION, and gvisor-tap-vsock from go.mod). Both
// release assets are universal Mach-O (arm64 + amd64), so one URL serves both.
const VFKIT_VERSION: &str = "0.6.1"; // crc-org/vfkit
const GVPROXY_VERSION: &str = "0.8.9"; // containers/gvisor-tap-vsock

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
            return Err(Error(format!("unsupported architecture: {other}")));
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
        other => Err(Error(format!("unsupported OS: {other}"))),
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
    .map_err(|e| Error(format!("bin lock: {e}")))?;

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
        .map_err(|e| Error(format!("bin lock release: {e}")))?;
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
            .map_err(|e| Error(format!("move partial install aside: {e}")))?;
    }
    // Best-effort sweep of leftover trash dirs / download temp files (this
    // run's renamed-aside tree, plus anything a crashed prior run left).
    // Safe — we hold the lock; prevents unbounded accumulation.
    sweep_leftovers(&podman_dir).await;

    tokio::fs::create_dir_all(root)
        .await
        .map_err(|e| Error(format!("mkdir {root:?}: {e}")))?;

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
        return Err(Error(format!(
            "podman executable not found after extract at {exe:?}"
        )));
    }

    // macOS: the official remote-client zip is podman + podman-mac-helper only.
    // `podman machine` (the applehv VM) ALSO needs vfkit (the VM monitor) and
    // gvproxy (networking), which podman does NOT auto-download — they ship in
    // the official .pkg. Fetch them beside podman so the setup phase's
    // `CONTAINERS_HELPER_BINARY_DIR` (= podman's dir) finds them.
    if std::env::consts::OS == "macos" {
        fetch_macos_helpers(exe).await?;
    }

    // Marker LAST — its presence is the signal that the install is COMPLETE.
    tokio::fs::write(marker, b"")
        .await
        .map_err(|e| Error(format!("write {marker:?}: {e}")))?;
    Ok(())
}

/// Fetch the macOS `podman machine` (applehv) helpers beside `exe`.
///
/// `vfkit` is the pre-SIGNED release asset: it carries the
/// `com.apple.security.virtualization` entitlement required to use
/// Virtualization.framework, so it is used AS-IS (re-signing would strip the
/// entitlement). `gvproxy` is ad-hoc codesigned (best-effort) so it runs on
/// Apple Silicon, which refuses unsigned binaries. Both assets are universal
/// Mach-O (arm64 + amd64). `krunkit` (the libkrun provider) is intentionally
/// omitted — it's only needed for `--provider libkrun`, not the default
/// applehv path.
async fn fetch_macos_helpers(exe: &Path) -> Result<(), Error> {
    let dir = exe
        .parent()
        .ok_or_else(|| Error(format!("podman exe has no parent: {exe:?}")))?;

    let vfkit = dir.join("vfkit");
    let vfkit_url =
        format!("https://github.com/crc-org/vfkit/releases/download/v{VFKIT_VERSION}/vfkit");
    download_to(&vfkit_url, &vfkit).await?;
    set_executable(&vfkit).await?;

    let gvproxy = dir.join("gvproxy");
    let gvproxy_url = format!(
        "https://github.com/containers/gvisor-tap-vsock/releases/download/v{GVPROXY_VERSION}/gvproxy-darwin"
    );
    download_to(&gvproxy_url, &gvproxy).await?;
    set_executable(&gvproxy).await?;
    // Apple Silicon refuses to exec unsigned binaries; ad-hoc sign gvproxy so
    // it runs regardless of how the upstream asset was signed (vfkit is left
    // untouched to preserve its entitlement signature). Best-effort.
    let _ = ad_hoc_codesign(&gvproxy).await;

    Ok(())
}

/// Set the unix executable bit (0o755). No-op on non-unix.
async fn set_executable(path: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .await
            .map_err(|e| Error(format!("chmod {path:?}: {e}")))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Ad-hoc codesign a binary (`codesign --force --sign - <path>`).
async fn ad_hoc_codesign(path: &Path) -> Result<(), Error> {
    let status = tokio::process::Command::new("codesign")
        .arg("--force")
        .arg("--sign")
        .arg("-")
        .arg(path)
        .status()
        .await
        .map_err(|e| Error(format!("spawn codesign: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error(format!("codesign failed for {path:?}")))
    }
}

/// Run the synchronous [`extract_archive`] on a blocking thread.
async fn run_extract(archive: &Path, root: &Path, kind: ArchiveKind) -> Result<(), Error> {
    let archive = archive.to_path_buf();
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || extract_archive(&archive, &root, kind))
        .await
        .map_err(|e| Error(format!("extract task join: {e}")))?
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
        .map_err(|e| Error(format!("open {archive:?}: {e}")))?;
    match kind {
        ArchiveKind::Zip => {
            let mut zip = zip::ZipArchive::new(file)
                .map_err(|e| Error(format!("read zip: {e}")))?;
            zip.extract(dest)
                .map_err(|e| Error(format!("extract zip: {e}")))?;
        }
        ArchiveKind::TarGz => {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut tar = tar::Archive::new(decoder);
            tar.unpack(dest)
                .map_err(|e| Error(format!("extract tar.gz: {e}")))?;
        }
    }
    Ok(())
}

/// Stream an HTTP GET to `dst`. Adapted from
/// `crate::command::update::download_to`.
async fn download_to(url: &str, dst: &Path) -> Result<(), Error> {
    use futures::StreamExt as _;
    use tokio::io::AsyncWriteExt as _;

    // NO timeout, deliberately. This is a tens-of-MB archive over
    // whatever link the user has, and a deadline here is a guess about
    // their bandwidth — one that turns a slow but healthy download into
    // a failed install. A dead connection surfaces as a transport
    // error; a slow one is allowed to finish.
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header(
            "User-Agent",
            format!("objectiveai/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .map_err(|e| Error(format!("http: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error(format!("download {url}: status {status}")));
    }

    let mut file = tokio::fs::File::create(dst)
        .await
        .map_err(|e| Error(format!("create {dst:?}: {e}")))?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| Error(format!("http: {e}")))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| Error(format!("write {dst:?}: {e}")))?;
    }
    file.flush()
        .await
        .map_err(|e| Error(format!("flush {dst:?}: {e}")))?;
    Ok(())
}
