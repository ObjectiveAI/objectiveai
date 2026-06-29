//! Laboratory containers: resolve a laboratory id to its running podman
//! container and the host port its MCP server is published on.
//!
//! A laboratory's container is named `objectiveai-laboratory-<state>-<id>`
//! (`<state>` from `ctx.filesystem.state()`), so the same id in different
//! objectiveai states maps to different containers. Inside the container the
//! laboratory MCP server listens on the fixed port [`LAB_PORT`]; that port is
//! published to a random `127.0.0.1` host port the caller looks up here.
//!
//! [`create`] is the container's source of truth — `podman create` → `podman
//! cp` (inject the lab MCP binary) → `podman start`. [`host_port`] then resolves
//! the published host port for the conduit to dial.

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

/// One host→container bind mount for `podman create -v host:container`.
pub struct Mount {
    pub host: String,
    pub container: String,
}

/// The `objectiveai.laboratory` container label — the authoritative round-trip
/// record of how a laboratory was created, so `list` can reconstruct its spec
/// without relying on podman's merged env/mount parsing. Env is a list of
/// `[key, value]` pairs.
#[derive(serde::Serialize)]
struct Label<'a> {
    id: &'a str,
    image: &'a str,
    mounts: Vec<LabelMount<'a>>,
    env: Vec<[&'a str; 2]>,
}

#[derive(serde::Serialize)]
struct LabelMount<'a> {
    host: &'a str,
    container: &'a str,
}

/// Create + start a laboratory container: `podman create` → `podman cp` (inject
/// the bundled `objectiveai-mcp-laboratory` musl binary) → `podman start`, in
/// that order so the injected binary exists before the entrypoint runs it.
///
/// The container is named [`container_name`]`(state, id)`, publishes its fixed
/// internal [`LAB_PORT`] to a random `127.0.0.1` host port (looked up later by
/// [`host_port`]), forces `PORT=14978` (appended after the user's env so it
/// wins), records its spec in the `objectiveai.laboratory` label, and overrides
/// the entrypoint to the injected MCP binary so container lifetime == MCP
/// lifetime.
pub async fn create(
    ctx: &Context,
    id: &str,
    image: &str,
    mounts: &[Mount],
    env: &[(String, String)],
) -> Result<(), Error> {
    let exe = ctx.podman().await?;
    let state = ctx.filesystem.state();
    let name = container_name(state, id);

    let label = Label {
        id,
        image,
        mounts: mounts
            .iter()
            .map(|m| LabelMount {
                host: &m.host,
                container: &m.container,
            })
            .collect(),
        env: env.iter().map(|(k, v)| [k.as_str(), v.as_str()]).collect(),
    };
    let label_json = serde_json::to_string(&label)
        .map_err(|e| Error::Podman(format!("serialize laboratory label: {e}")))?;

    // 1. podman create
    let mut create_cmd = container_command(exe);
    create_cmd
        .arg("create")
        .arg("--name")
        .arg(&name)
        .arg("-p")
        .arg(format!("127.0.0.1::{LAB_PORT}/tcp"));
    for m in mounts {
        create_cmd
            .arg("-v")
            .arg(format!("{}:{}", m.host, m.container));
    }
    for (k, v) in env {
        create_cmd.arg("-e").arg(format!("{k}={v}"));
    }
    // Force the MCP's bind port; appended after the user's env so it wins.
    create_cmd.arg("-e").arg(format!("PORT={LAB_PORT}"));
    create_cmd
        .arg("--label")
        .arg(format!("objectiveai.laboratory={label_json}"))
        .arg("--entrypoint")
        .arg("/objectiveai-mcp-laboratory")
        .arg(image);
    let output = create_cmd
        .output()
        .await
        .map_err(|e| Error::Podman(format!("spawn podman create: {e}")))?;
    if !output.status.success() {
        return Err(Error::Podman(format!(
            "podman create {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    // 2. podman cp — inject the bundled musl MCP binary (no `.exe`, ever).
    let bin = ctx.filesystem.bin_dir().join("objectiveai-mcp-laboratory");
    let output = container_command(exe)
        .arg("cp")
        .arg(&bin)
        .arg(format!("{name}:/objectiveai-mcp-laboratory"))
        .output()
        .await
        .map_err(|e| Error::Podman(format!("spawn podman cp: {e}")))?;
    if !output.status.success() {
        return Err(Error::Podman(format!(
            "podman cp into {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    // 3. podman start — runs the injected entrypoint with PORT=14978.
    let output = container_command(exe)
        .arg("start")
        .arg(&name)
        .output()
        .await
        .map_err(|e| Error::Podman(format!("spawn podman start: {e}")))?;
    if !output.status.success() {
        return Err(Error::Podman(format!(
            "podman start {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
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
