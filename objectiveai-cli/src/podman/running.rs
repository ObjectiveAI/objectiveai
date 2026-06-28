//! Podman machine **liveness** — ensure the global machine is *running*.
//!
//! [`ensure_running`] converges the global machine ([`super::setup::MACHINE_NAME`])
//! to a running state regardless of prior state: not yet created, created but
//! stopped (e.g. after a host reboot), already running, or even
//! created-but-since-destroyed. Linux runs containers natively — no machine —
//! so it is a no-op there.
//!
//! Unlike [`super::install`] / the old setup, this gates on a **live probe**
//! (`podman machine inspect`), not a durable completion marker: a machine's
//! running-ness is volatile (a reboot stops it) and cheaply queryable, so a
//! marker would only ever go stale and hand back a dead machine. The probe is
//! also self-healing — a destroyed-but-previously-initialized machine reads as
//! `Absent` and is re-created.
//!
//! Concurrency mirrors [`super::install`]: the lock-free fast path returns
//! immediately when the machine is already running; otherwise the reconcile is
//! serialized by the bin lock (`<bin>/locks`, key `podman-machine`) with
//! double-checked locking (probe → `wait_acquire` → re-probe → reconcile →
//! explicit release). Across DIFFERENT objectiveai dirs the per-dir lock does
//! not serialize, but the global machine still converges: each reconcile
//! re-probes, and podman's "already exists" / "already running" are treated as
//! success.

use std::path::Path;

use crate::error::Error;
use crate::podman::setup::{self, MACHINE_NAME};

/// The global machine's lifecycle state, as read by [`machine_state`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum MachineState {
    /// No such machine — it has never been created (or was destroyed).
    Absent,
    /// The machine exists but is not running (e.g. after a host reboot).
    Stopped,
    /// The machine is up and ready to serve container ops.
    Running,
}

/// Ensure the global podman machine is **running** for this host, creating it
/// first if necessary. No-op on Linux (native rootless podman needs no
/// machine). Concurrency-safe; see the module docs. `exe` is the podman binary
/// from [`super::install`].
pub async fn ensure_running(bin_dir: &Path, exe: &Path) -> Result<(), Error> {
    // Linux runs containers natively — no machine to start.
    if std::env::consts::OS == "linux" {
        return Ok(());
    }
    let helper_dir = exe.parent();

    // 1. Fast path: already running — no lock.
    if machine_state(exe, helper_dir).await == MachineState::Running {
        return Ok(());
    }

    // 2. Serialize the reconcile machine-wide. We may BLOCK here while a
    //    sibling process is mid-init/start (or about to finish one).
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &bin_dir.join("locks"),
        "podman-machine",
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| Error::Podman(format!("machine lock: {e}")))?;

    // 3. DOUBLE-CHECKED LOCKING: re-probe now that we hold the lock — a sibling
    //    may have started (or created) the machine while we were blocked.
    let result = reconcile(exe, helper_dir).await;

    // 4. Release explicitly on EVERY path — dropping a LockClaim deliberately
    //    does NOT release it (mirrors `install::ensure_installed`). Release
    //    before propagating an error so the lock is never leaked.
    claim
        .release()
        .map_err(|e| Error::Podman(format!("machine lock release: {e}")))?;
    result
}

/// Drive the machine to [`MachineState::Running`] from whatever it is now.
/// Caller holds the `podman-machine` bin lock.
async fn reconcile(exe: &Path, helper_dir: Option<&Path>) -> Result<(), Error> {
    match machine_state(exe, helper_dir).await {
        MachineState::Running => Ok(()),
        MachineState::Stopped => machine_start(exe, helper_dir).await,
        // `machine init` does NOT auto-start (no `--now`), so start after it.
        MachineState::Absent => {
            setup::machine_init(exe, helper_dir).await?;
            machine_start(exe, helper_dir).await
        }
    }
}

/// Probe the global machine's lifecycle state via `podman machine inspect`.
/// A failed inspect (no such machine) reads as [`MachineState::Absent`];
/// `State == "running"` is [`MachineState::Running`]; anything else is
/// [`MachineState::Stopped`].
async fn machine_state(exe: &Path, helper_dir: Option<&Path>) -> MachineState {
    let output = setup::command(exe, helper_dir)
        .arg("machine")
        .arg("inspect")
        .arg(MACHINE_NAME)
        .arg("--format")
        .arg("{{.State}}")
        .output()
        .await;
    let Ok(output) = output else {
        // Spawn failure — can't tell; treat as Stopped so we attempt a start
        // under the lock (which surfaces a real error if podman is broken).
        return MachineState::Stopped;
    };
    if !output.status.success() {
        return MachineState::Absent;
    }
    if String::from_utf8_lossy(&output.stdout).trim() == "running" {
        MachineState::Running
    } else {
        MachineState::Stopped
    }
}

/// Start the global machine (`podman machine start objectiveai`). A machine
/// that another objectiveai dir started between our probe and this call reports
/// "already running"; treat that as success.
async fn machine_start(exe: &Path, helper_dir: Option<&Path>) -> Result<(), Error> {
    let output = setup::command(exe, helper_dir)
        .arg("machine")
        .arg("start")
        .arg(MACHINE_NAME)
        .output()
        .await
        .map_err(|e| Error::Podman(format!("spawn podman machine start: {e}")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if stderr.contains("already running") || stderr.contains("already started") {
        return Ok(());
    }
    Err(Error::Podman(format!(
        "podman machine start failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}
