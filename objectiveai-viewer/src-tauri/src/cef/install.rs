//! The Chromium runtime, acquired on demand.
//!
//! Linking against CEF does NOT bundle Chromium: `libcef.dll` plus its
//! resource pak files and locales are a ~200MB tree that has to be
//! somewhere on disk before the first browser tab opens. Shipping it
//! inside the viewer would tax every user who never opens one, so
//! [`ensure_runtime`] fetches it the first time one does — and that is
//! only possible because `build.rs` delay-loads `libcef.dll`: a viewer
//! that has never spawned a browser tab starts with no runtime present
//! and never touches a CEF symbol.
//!
//! Two sources, probed in order:
//!
//! 1. **Beside the executable.** `cef-dll-sys`'s build script copies
//!    the runtime into the cargo target dir, so a `cargo run`/`tauri
//!    dev` viewer already has it — and a packaged viewer that ships it
//!    likewise. Nothing to download.
//! 2. **`<bin>/cef/<version>/`**, downloaded from the same CDN and at
//!    the same pinned version the build linked against.
//!
//! The download mirrors `objectiveai-laboratory`'s podman installer
//! exactly: probe marker → `wait_acquire` the bin lock → re-probe →
//! download + extract → write `.objectiveai-install-complete` → release
//! explicitly. A partial install (dir present, marker absent) is
//! renamed aside and redone, because Windows directory deletion is
//! asynchronous and a delete-then-recreate races ACCESS_DENIED.

use std::path::{Path, PathBuf};

/// The CEF build the viewer LINKS against — it must equal the build
/// metadata of the `cef` dependency in `Cargo.toml` (`150.2.1+150.0.14`
/// ⇒ `150.0.14`). A downloaded runtime older or newer than the headers
/// we compiled against will fail the `api_hash` handshake at
/// [`super::initialize`], so bump both together.
pub const CEF_VERSION: &str = "150.0.14";

/// The DLL whose presence means "a usable runtime lives in this
/// directory" — the delay-loaded import everything else hangs off.
#[cfg(target_os = "windows")]
const RUNTIME_MARKER: &str = "libcef.dll";
#[cfg(target_os = "linux")]
const RUNTIME_MARKER: &str = "libcef.so";
#[cfg(target_os = "macos")]
const RUNTIME_MARKER: &str = "Chromium Embedded Framework.framework";

/// Download timeout for the (hundreds-of-MB) CEF archive.
const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1800);

/// Where the runtime was found, and whether it had to be fetched.
pub struct Runtime {
    /// The directory holding `libcef.dll`, the `.pak` resources, the
    /// `locales/` subdirectory, and (Windows/Linux) nothing else we
    /// need — CEF's `resources_dir_path` and `locales_dir_path` are
    /// both derived from it.
    pub dir: PathBuf,
}

/// Resolve a usable CEF runtime, downloading it once if neither the
/// executable's own directory nor `<bin>/cef/<version>/` has one.
///
/// `bin_dir` is the layout's `bin` directory (`<objectiveai_dir>/bin`)
/// — the same one podman and the pinned Python install under, and the
/// same one whose `locks/` subdirectory serializes those installs.
pub async fn ensure_runtime(bin_dir: PathBuf) -> Result<Runtime, String> {
    // 1. Beside the exe: what a `cargo`-built or self-contained viewer
    //    already has. No lock, no marker — the build staged it whole.
    if let Some(dir) = beside_exe().await {
        return Ok(Runtime { dir });
    }

    let root = bin_dir.join("cef").join(CEF_VERSION);
    let dir = root.join(extracted_dir_name());
    let marker = root.join(".objectiveai-install-complete");

    // 2. A completed download — no lock.
    if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
        return Ok(Runtime { dir });
    }

    // 3. Serialize machine-wide. We may BLOCK here while a sibling
    //    process is mid-download.
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &bin_dir.join("locks"),
        "cef",
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| format!("bin lock: {e}"))?;

    // 4. Double-checked: a sibling may have finished while we blocked.
    let result = async {
        if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
            return Ok(());
        }
        install(&root, &marker, &dir).await
    }
    .await;

    // 5. Release on EVERY path — dropping a LockClaim deliberately does
    //    NOT release it — and before propagating an install error, so a
    //    failed download never leaks the lock.
    claim
        .release()
        .map_err(|e| format!("bin lock release: {e}"))?;
    result?;
    Ok(Runtime { dir })
}

/// The runtime directory beside the current executable, if it holds
/// one. `cef-dll-sys` copies the runtime into the cargo target dir at
/// build time, which is exactly where a `tauri dev` binary runs from.
async fn beside_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    tokio::fs::try_exists(dir.join(RUNTIME_MARKER))
        .await
        .unwrap_or(false)
        .then_some(dir)
}

/// The single directory `download_cef::extract_target_archive` leaves
/// behind — it renames the archive's `Release/` to `cef_<os>_<arch>/`
/// and merges `Resources/` into it.
fn extracted_dir_name() -> String {
    download_cef::OsAndArch::try_from(download_cef::DEFAULT_TARGET)
        .map(|oa| oa.to_string())
        // `DEFAULT_TARGET` is chosen by the same crate's cfgs, so the
        // conversion cannot actually fail; the fallback keeps this
        // infallible rather than panicking a browser spawn.
        .unwrap_or_else(|_| "cef".to_string())
}

/// Download + extract into `root`. The caller holds the bin lock and
/// has already re-checked (under the lock) that the install is missing.
async fn install(root: &Path, marker: &Path, dir: &Path) -> Result<(), String> {
    let cef_dir = root
        .parent()
        .ok_or_else(|| format!("cef root has no parent: {root:?}"))?
        .to_path_buf();

    // A partial extract has the dir but no marker. Rename aside rather
    // than delete in place — see the module docs.
    if tokio::fs::try_exists(root).await.unwrap_or(false) {
        let trash = cef_dir.join(format!(
            "{CEF_VERSION}.trash-{}",
            std::process::id()
        ));
        tokio::fs::rename(root, &trash)
            .await
            .map_err(|e| format!("move partial cef install aside: {e}"))?;
    }
    sweep_leftovers(&cef_dir).await;

    tokio::fs::create_dir_all(root)
        .await
        .map_err(|e| format!("mkdir {root:?}: {e}"))?;

    // `download-cef` is the same crate `cef-dll-sys`'s build script
    // uses, driven with the same target and version — so the runtime we
    // fetch is bit-for-bit the one the build linked against. It is
    // blocking (a synchronous reqwest + a bz2/tar extract), so it rides
    // a blocking thread.
    let root_owned = root.to_path_buf();
    let fetched = tokio::time::timeout(
        DOWNLOAD_TIMEOUT,
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let archive = download_cef::download_target_archive(
                download_cef::DEFAULT_TARGET,
                CEF_VERSION,
                &root_owned,
                false,
            )
            .map_err(|e| format!("download cef {CEF_VERSION}: {e}"))?;
            download_cef::extract_target_archive(
                download_cef::DEFAULT_TARGET,
                &archive,
                &root_owned,
                false,
            )
            .map_err(|e| format!("extract cef {CEF_VERSION}: {e}"))?;
            // The archive is the larger half of the disk cost and
            // nothing reads it again.
            let _ = std::fs::remove_file(&archive);
            Ok(())
        }),
    )
    .await
    .map_err(|_| format!("cef download timed out after {DOWNLOAD_TIMEOUT:?}"))?
    .map_err(|e| format!("cef download task join: {e}"))?;
    fetched?;

    // Guard against upstream layout drift: the runtime must be where
    // the extractor is documented to put it.
    if !tokio::fs::try_exists(dir.join(RUNTIME_MARKER))
        .await
        .unwrap_or(false)
    {
        return Err(format!(
            "cef runtime not found after extract at {:?}",
            dir.join(RUNTIME_MARKER)
        ));
    }

    // Marker LAST — its presence IS the completion signal.
    tokio::fs::write(marker, b"")
        .await
        .map_err(|e| format!("write {marker:?}: {e}"))?;
    Ok(())
}

/// Best-effort removal of `*.trash-*` leftovers under the cef dir —
/// this run's rename-aside plus anything a crashed prior run left. The
/// caller holds the lock, so this is race-free.
async fn sweep_leftovers(cef_dir: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(cef_dir).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        if !name.to_string_lossy().contains(".trash-") {
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

/// Make the delay-loaded `libcef.dll` resolvable out of `dir`.
///
/// Windows only, and the ONE thing that makes a FETCHED runtime usable:
/// the delay-load stub resolves `libcef.dll` by the ordinary search
/// order, which has no reason to include `<bin>/cef/<version>/`.
///
/// It does that by LOADING the DLL by absolute path, so the module is
/// already in the process under the base name the stub asks for and no
/// search ever happens. `LOAD_WITH_ALTERED_SEARCH_PATH` makes CEF's own
/// directory the search root for ITS dependencies — the ANGLE and
/// Vulkan DLLs that ship beside it.
///
/// It deliberately does NOT touch the process-wide search order
/// (`SetDefaultDllDirectories`/`AddDllDirectory`). That would apply to
/// every later `LoadLibrary` in the process, and this runs in Chromium
/// HELPER processes too — including the GPU process, which loads
/// graphics drivers by name and has no business inheriting an
/// embedder's search-order preferences.
///
/// A no-op when the runtime is already beside the executable (`cargo`
/// builds, and any packaging that ships it): the ordinary search finds
/// it, and doing nothing keeps that case a stock CEF embedding.
#[cfg(target_os = "windows")]
pub fn preload_runtime(dir: &Path) {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::System::LibraryLoader::{
        LOAD_WITH_ALTERED_SEARCH_PATH, LoadLibraryExW,
    };

    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent == dir))
        .unwrap_or(false);
    if beside_exe {
        return;
    }
    let dll = dir.join(RUNTIME_MARKER);
    let wide: Vec<u16> = dll
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Best-effort: a failure surfaces as the delay-load stub's own
    // "module not found" at the first CEF call, which names the DLL.
    unsafe {
        LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), LOAD_WITH_ALTERED_SEARCH_PATH);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn preload_runtime(_dir: &Path) {}
