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
//! Concurrency is two-layered. The lock-free fast path returns immediately when
//! the machine is already running. Otherwise the slow path is serialized at two
//! levels: an **in-process** `Mutex` (the bin lock below is a cross-PROCESS
//! mutex that is reentrant in-process, so it does NOT serialize concurrent
//! in-process callers — e.g. the conduit dialing several laboratories at once —
//! and without the `Mutex` they could both reconcile and double-start), then
//! the **cross-process** bin lock (`<bin>/locks`, key `podman-machine`) with
//! double-checked locking (probe → in-process `Mutex` → `wait_acquire` →
//! re-probe → reconcile → explicit release). Across DIFFERENT objectiveai dirs
//! the per-dir lock does not serialize, but the global machine still converges:
//! each reconcile re-probes, and podman's "already exists" / "already running"
//! are treated as success.

use std::path::Path;

use super::Error;
use super::setup::{self, MACHINE_NAME};

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
/// machine). Concurrency-safe; see the module docs. `machine_lock` is the
/// process-wide mutex (held by [`super::Podman`]) that serializes the
/// in-process slow path; `exe` is the podman binary from [`super::install`].
pub async fn ensure_running(
    machine_lock: &tokio::sync::Mutex<()>,
    bin_dir: &Path,
    exe: &Path,
) -> Result<(), Error> {
    // Linux runs containers natively — no machine to start.
    if std::env::consts::OS == "linux" {
        return Ok(());
    }
    let helper_dir = exe.parent();

    // 1. Fast path: already running — no lock at all.
    if machine_state(exe, helper_dir).await == MachineState::Running {
        return Ok(());
    }

    // 2. Serialize the slow path WITHIN this process. The cross-process bin
    //    lock below is reentrant in-process (it's a cross-PROCESS mutex,
    //    refcounted per process), so without this two concurrent in-process
    //    callers could both reconcile and double-`machine start`. Held for the
    //    whole reconcile; the fast path above stays lock-free.
    let _in_process = machine_lock.lock().await;

    // 3. Serialize the reconcile machine-WIDE. We may BLOCK here while a
    //    sibling process is mid-init/start (or about to finish one).
    let claim = crate::lockfile::wait_acquire(
        &bin_dir.join("locks"),
        "podman-machine",
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| Error(format!("machine lock: {e}")))?;

    // 4. DOUBLE-CHECKED LOCKING: `reconcile` re-probes now that we hold both
    //    locks — a sibling (process OR, before we got the Mutex, an in-process
    //    task) may have started or created the machine while we were blocked.
    let result = reconcile(exe, helper_dir).await;

    // 5. Release the bin lock explicitly on EVERY path — dropping a LockClaim
    //    deliberately does NOT release it (mirrors `install::ensure_installed`).
    //    Release before propagating an error so the lock is never leaked. The
    //    in-process Mutex guard drops at end of scope.
    claim
        .release()
        .map_err(|e| Error(format!("machine lock release: {e}")))?;
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
        .map_err(|e| Error(format!("spawn podman machine start: {e}")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if stderr.contains("already running") || stderr.contains("already started") {
        return Ok(());
    }
    Err(Error(format!(
        "podman machine start failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}
