//! Podman setup — phase 2, after [`super::install`].
//!
//! [`ensure_setup`] initializes the preconditions for *using* podman:
//! `podman machine init` (the macOS VM / Windows WSL2 distro). Linux runs
//! containers natively on the host, so its setup is a no-op.
//!
//! The machine is **global**, named [`MACHINE_NAME`] (`objectiveai`), shared
//! by every objectiveai dir on the host — matching Linux's already-global
//! container state. The completion marker, by contrast, is per-objectiveai-dir
//! (it lives under `<bin>`), so a fresh dir can find its marker missing even
//! though the global machine already exists (created by another dir). Hence
//! the existence check before `init`: if the machine is already there, just
//! write the marker.
//!
//! Concurrency mirrors [`super::install`] / `objectiveai-db`: in-process
//! callers coalesce on the `Context`'s `OnceCell`; across processes the setup
//! is serialized by the bin lock (`<bin>/locks`, key `podman-setup`) and gated
//! by the `.objectiveai-setup-complete` marker (probe → `wait_acquire` →
//! re-probe → init → marker → explicit release). Across DIFFERENT objectiveai
//! dirs the per-dir lock does not serialize, but the global machine still
//! converges safely: the existence check skips a redundant init, and a
//! genuine cross-dir `init` race is absorbed by treating podman's
//! "already exists" as success (podman itself serializes machine creation).

use std::path::Path;

use crate::error::Error;

/// The single global podman machine name, shared by every objectiveai dir on
/// the host. Public so the future "use podman" code targets it
/// (`podman --connection objectiveai ...`).
pub const MACHINE_NAME: &str = "objectiveai";

/// Ensure the global podman machine is initialized for this host.
///
/// macOS/Windows → `podman machine init objectiveai` (which downloads the
/// VM/WSL image — podman's concern, not ours). Linux → no-op (native rootless
/// podman needs no machine). Concurrency-safe; see the module docs. `exe` is
/// the podman binary from [`super::install`].
pub async fn ensure_setup(bin_dir: &Path, exe: &Path) -> Result<(), Error> {
    let marker = bin_dir.join("podman").join(".objectiveai-setup-complete");

    // 1. Fast path: a completed setup — no lock.
    if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
        return Ok(());
    }

    // 2. Serialize setup within this dir (a sibling may be mid-init, or have
    //    finished while we were blocked acquiring).
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &bin_dir.join("locks"),
        "podman-setup",
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| Error::Podman(format!("setup lock: {e}")))?;

    // 3. DOUBLE-CHECKED LOCKING: re-check the marker under the lock, then run
    //    setup if still missing. Runs entirely under the lock.
    let result = async {
        if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
            return Ok(());
        }
        machine_init(exe).await?;
        tokio::fs::write(&marker, b"")
            .await
            .map_err(|e| Error::Podman(format!("write {marker:?}: {e}")))
    }
    .await;

    // 4. Release explicitly on every path (dropping a LockClaim does NOT
    //    release it); release before propagating an error so it never leaks.
    claim
        .release()
        .map_err(|e| Error::Podman(format!("setup lock release: {e}")))?;
    result
}

/// Initialize the global podman machine if it doesn't already exist.
/// No-op on Linux (native). Caller holds the setup lock.
async fn machine_init(exe: &Path) -> Result<(), Error> {
    // Linux runs containers natively — no machine to create.
    if std::env::consts::OS == "linux" {
        return Ok(());
    }

    // The bundled Windows helpers (gvproxy.exe / win-sshproxy.exe) sit beside
    // podman.exe; point podman at that dir so it finds them.
    let helper_dir = exe.parent().map(Path::to_path_buf);

    // The machine may already exist on this host — created by another
    // objectiveai dir (whose marker we don't share), or left from a previous
    // run whose marker was lost. If so, skip init.
    if machine_exists(exe, helper_dir.as_deref()).await {
        return Ok(());
    }

    // `init` downloads a large image — no short timeout; the lock serializes
    // in-dir callers.
    let output = command(exe, helper_dir.as_deref())
        .arg("machine")
        .arg("init")
        .arg(MACHINE_NAME)
        .output()
        .await
        .map_err(|e| Error::Podman(format!("spawn podman machine init: {e}")))?;
    if output.status.success() {
        return Ok(());
    }

    // A cross-dir race: another process created the machine between our
    // existence check and this init. podman reports it; treat as success.
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.to_ascii_lowercase().contains("already exists") {
        return Ok(());
    }
    Err(Error::Podman(format!(
        "podman machine init failed: {}",
        stderr.trim()
    )))
}

/// Whether the global machine already exists (`podman machine inspect` exits 0).
async fn machine_exists(exe: &Path, helper_dir: Option<&Path>) -> bool {
    command(exe, helper_dir)
        .arg("machine")
        .arg("inspect")
        .arg(MACHINE_NAME)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A `podman` command with the bundled helper dir wired in.
fn command(exe: &Path, helper_dir: Option<&Path>) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(exe);
    if let Some(dir) = helper_dir {
        cmd.env("CONTAINERS_HELPER_BINARY_DIR", dir);
    }
    cmd
}
