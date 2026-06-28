//! Podman machine **init** — the macOS VM / Windows WSL2 distro.
//!
//! This module owns creating the global machine ([`machine_init`]); making it
//! *running* is [`super::running`]'s job. The machine is **global**, named
//! [`MACHINE_NAME`] (`objectiveai`), shared by every objectiveai dir on the
//! host — matching Linux's already-global container state. Linux runs
//! containers natively, so it has no machine and never calls in here.
//!
//! [`machine_init`] is a raw, lock-free primitive: the reconcile in
//! [`super::running`] only calls it once it has probed the machine `Absent`,
//! holding the `podman-machine` bin lock. A genuine cross-dir `init` race
//! (another objectiveai dir creating the machine between the probe and the
//! init) is absorbed by treating podman's "already exists" as success — podman
//! itself serializes machine creation.

use std::path::Path;

use crate::error::Error;

/// The single global podman machine name, shared by every objectiveai dir on
/// the host. Public so the "use podman" code targets it
/// (`podman --connection objectiveai ...`).
pub const MACHINE_NAME: &str = "objectiveai";

/// Create the global podman machine (`podman machine init objectiveai`).
///
/// Raw primitive — no existence pre-check, no lock: the caller
/// ([`super::running::ensure_running`]) has already probed the machine `Absent`
/// under the `podman-machine` lock. `machine init` downloads a large VM/WSL
/// image, so no short timeout is imposed; the caller's lock serializes in-dir
/// callers. `exe` is the podman binary from [`super::install`]; `helper_dir`
/// (podman's own dir) is wired in so it finds the bundled machine helpers.
pub(crate) async fn machine_init(exe: &Path, helper_dir: Option<&Path>) -> Result<(), Error> {
    let output = command(exe, helper_dir)
        .arg("machine")
        .arg("init")
        .arg(MACHINE_NAME)
        .output()
        .await
        .map_err(|e| Error::Podman(format!("spawn podman machine init: {e}")))?;
    if output.status.success() {
        return Ok(());
    }

    // Cross-dir race: another process created the machine between the caller's
    // existence probe and this init. podman reports it; treat as success.
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.to_ascii_lowercase().contains("already exists") {
        return Ok(());
    }
    Err(Error::Podman(format!(
        "podman machine init failed: {}",
        stderr.trim()
    )))
}

/// A `podman` command with the bundled helper dir wired in
/// (`CONTAINERS_HELPER_BINARY_DIR`) so podman finds the machine helpers that
/// ship beside it (`gvproxy`/`win-sshproxy` on Windows, `vfkit`/`gvproxy` on
/// macOS).
pub(crate) fn command(exe: &Path, helper_dir: Option<&Path>) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(exe);
    if let Some(dir) = helper_dir {
        cmd.env("CONTAINERS_HELPER_BINARY_DIR", dir);
    }
    cmd
}
