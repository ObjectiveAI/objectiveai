//! Laboratory containers: resolve a laboratory id to its running podman
//! container and the host port its MCP server is published on.
//!
//! A laboratory's container is named `objectiveai-laboratory-<state>-<id>`
//! (`<state>` from `ctx.filesystem.state()`), so the same id in different
//! objectiveai states maps to different containers. Inside the container the
//! laboratory MCP server listens on the fixed port [`LAB_PORT`]; that port is
//! published to a random `127.0.0.1` host port the caller looks up here.
//!
//! NOTE: nothing creates these containers yet — that's future work, so
//! [`host_port`] will error for any attached laboratory until then.

use std::path::Path;

use crate::context::Context;
use crate::error::Error;
use crate::podman::setup::MACHINE_NAME;

/// Fixed port the laboratory MCP server listens on inside its container.
pub const LAB_PORT: u16 = 14978;

/// Per-state container name for a laboratory id.
pub fn container_name(state: &str, id: &str) -> String {
    format!("objectiveai-laboratory-{state}-{id}")
}

/// A `podman` command for laboratory CONTAINER operations. On non-Linux the
/// containers live inside the `objectiveai` machine, so target that
/// connection; on Linux they run natively (no `--connection`). Wires
/// `CONTAINERS_HELPER_BINARY_DIR` to podman's own dir, like podman/setup.rs.
fn container_command(exe: &Path) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(exe);
    if let Some(dir) = exe.parent() {
        cmd.env("CONTAINERS_HELPER_BINARY_DIR", dir);
    }
    if std::env::consts::OS != "linux" {
        cmd.arg("--connection").arg(MACHINE_NAME);
    }
    cmd
}

/// The `127.0.0.1` host port the container's [`LAB_PORT`]/tcp is published on.
pub async fn host_port(ctx: &Context, id: &str) -> Result<u16, Error> {
    let exe = ctx.podman().await?;
    let name = container_name(ctx.filesystem.state(), id);
    let output = container_command(exe)
        .arg("port")
        .arg(&name)
        .arg(format!("{LAB_PORT}/tcp"))
        .output()
        .await
        .map_err(|e| Error::Podman(format!("spawn podman port: {e}")))?;
    if !output.status.success() {
        return Err(Error::Podman(format!(
            "podman port {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    // Output is one or more lines like `127.0.0.1:49160`; take the first
    // non-empty line and the port after the last ':'.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .ok_or_else(|| {
            Error::Podman(format!("podman port {name}: no mapping for {LAB_PORT}/tcp"))
        })?;
    let port_str = line.rsplit_once(':').map(|(_, p)| p).unwrap_or(line);
    port_str.parse::<u16>().map_err(|e| {
        Error::Podman(format!(
            "podman port {name}: unparseable host port {port_str:?}: {e}"
        ))
    })
}
