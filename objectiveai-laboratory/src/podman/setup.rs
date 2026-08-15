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

use super::Error;

/// The single global podman machine name, shared by every objectiveai dir on
/// the host. Public so the "use podman" code targets it
/// (`podman --connection objectiveai ...`).
pub const MACHINE_NAME: &str = "objectiveai";

/// Memory (MiB) for the macOS machine. Podman's applehv default is 2048,
/// which is not enough to compile the official Rust plugin scaffold — rustc
/// is OOM-killed building `starlark`, and the kernel's SIGKILL surfaces four
/// layers away as an MCP 502 that reads as a network fault. 6144 is the
/// measured floor that builds it. Linux runs containers natively (no
/// machine), and only macOS has the measured failure, so only macOS is
/// gated onto this value.
pub(crate) const MACHINE_MEMORY_MIB: u32 = 6144;

/// Create the global podman machine (`podman machine init objectiveai`).
///
/// Raw primitive — no existence pre-check, no lock: the caller
/// ([`super::running::ensure_running`]) has already probed the machine `Absent`
/// under the `podman-machine` lock. `machine init` downloads a large VM/WSL
/// image, so no short timeout is imposed; the caller's lock serializes in-dir
/// callers. `exe` is the podman binary from [`super::install`]; `helper_dir`
/// (podman's own dir) is wired in so it finds the bundled machine helpers.
pub(crate) async fn machine_init(exe: &Path, helper_dir: Option<&Path>) -> Result<(), Error> {
    let mut cmd = command(exe, helper_dir);
    cmd.arg("machine").arg("init").arg(MACHINE_NAME);
    // See MACHINE_MEMORY_MIB — without this, a macOS machine gets podman's
    // 2 GiB default and every plugin-scaffold build inside it is OOM-killed.
    if std::env::consts::OS == "macos" {
        cmd.arg("--memory").arg(MACHINE_MEMORY_MIB.to_string());
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman machine init: {e}")))?;
    if output.status.success() {
        return Ok(());
    }

    // Cross-dir race: another process created the machine between the caller's
    // existence probe and this init. podman reports it; treat as success.
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.to_ascii_lowercase().contains("already exists") {
        return Ok(());
    }
    Err(Error(format!(
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
    objectiveai_sdk::process::no_window(&mut cmd);
    cmd
}
