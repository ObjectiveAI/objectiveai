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

/// Default working directory new agents start in, when `create` is called
/// without an explicit cwd and for old containers whose label predates the
/// field.
fn default_cwd() -> String {
    "/".to_string()
}

/// A laboratory container as read back by [`list`], reconstructed from its
/// `objectiveai.laboratory` label. Mirrors the `create` echo.
pub struct LaboratoryInfo {
    pub id: String,
    pub image: String,
    pub mounts: Vec<Mount>,
    pub env: Vec<(String, String)>,
    pub cwd: String,
}

/// The `objectiveai.laboratory` container label — the authoritative round-trip
/// record of how a laboratory was created, so `list` can reconstruct its spec
/// without relying on podman's merged env/mount parsing. Env is a list of
/// `[key, value]` pairs.
#[derive(serde::Serialize, serde::Deserialize)]
struct Label {
    id: String,
    image: String,
    mounts: Vec<LabelMount>,
    env: Vec<[String; 2]>,
    /// Default working directory for new agents. `#[serde(default)]` so
    /// containers created before this field round-trip in [`list`] as `/`.
    #[serde(default = "default_cwd")]
    cwd: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LabelMount {
    host: String,
    container: String,
}

/// Create a laboratory container (created, NOT started): `podman create` →
/// `podman cp` (inject the bundled `objectiveai-mcp-laboratory` musl binary).
/// Starting it is done elsewhere.
///
/// The container is named [`container_name`]`(state, id)`, publishes its fixed
/// internal [`LAB_PORT`] to a random `127.0.0.1` host port (looked up later by
/// [`host_port`]), forces `PORT=14978` (appended after the user's env so it
/// wins), bakes in the default working directory new agents start in
/// (`OBJECTIVEAI_LABORATORY_CWD=<cwd>`), records its spec in the
/// `objectiveai.laboratory` label, and overrides the entrypoint to a shell
/// wrapper that `chmod +x`es then exec's the injected MCP binary (so the
/// `podman cp`'d binary is executable regardless of the host's file mode, and
/// the container lifetime == MCP lifetime).
pub async fn create(
    ctx: &Context,
    id: &str,
    image: &str,
    mounts: &[Mount],
    env: &[(String, String)],
    cwd: &str,
) -> Result<(), Error> {
    let exe = ctx.podman().await?;
    let state = ctx.filesystem.state();
    let name = container_name(state, id);

    let label = Label {
        id: id.to_string(),
        image: image.to_string(),
        mounts: mounts
            .iter()
            .map(|m| LabelMount {
                host: m.host.clone(),
                container: m.container.clone(),
            })
            .collect(),
        env: env.iter().map(|(k, v)| [k.clone(), v.clone()]).collect(),
        cwd: cwd.to_string(),
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
    // The laboratory id, so the in-container MCP names itself `oail-<id>`.
    // Baked into the container config at create time → static and persists
    // across restarts. Appended after the user's env so it wins.
    create_cmd
        .arg("-e")
        .arg(format!("OBJECTIVEAI_LABORATORY_ID={id}"));
    // The default working directory new agents start in. Appended after the
    // user's env so it wins.
    create_cmd
        .arg("-e")
        .arg(format!("OBJECTIVEAI_LABORATORY_CWD={cwd}"));
    create_cmd
        .arg("--label")
        .arg(format!("objectiveai.laboratory={label_json}"))
        // Entrypoint is a shell wrapper that `chmod +x`es the injected binary
        // before exec'ing it. `podman cp` (below) preserves the *host* file
        // mode, and on Windows/macOS the bundled musl binary arrives without a
        // Unix execute bit — so a bare `--entrypoint /objectiveai-mcp-laboratory`
        // fails at start with "exists but it is not executable". The chmod is
        // idempotent and runs on every start; `exec` keeps the MCP as PID 1 so
        // container lifetime == MCP lifetime. A laboratory image always has a
        // shell (its raison d'être is running the Bash tool).
        .arg("--entrypoint")
        .arg(
            r#"["/bin/sh","-c","chmod +x /objectiveai-mcp-laboratory && exec /objectiveai-mcp-laboratory"]"#,
        )
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

    // NOTE: the container is created but NOT started here — starting it (which
    // runs the injected entrypoint on PORT=14978) is done elsewhere (see
    // [`start`], called by the conduit at dial time).
    Ok(())
}

/// Start a laboratory container, idempotently. `podman start` is a no-op that
/// still exits 0 when the container is already running, and podman serializes
/// container ops internally — so this is safe to run BLINDLY and CONCURRENTLY
/// (two parallel starters both succeed; no check-then-start race). Errors only
/// if the container does not exist (the id was never [`create`]d).
pub async fn start(ctx: &Context, id: &str) -> Result<(), Error> {
    let exe = ctx.podman().await?;
    let name = container_name(ctx.filesystem.state(), id);
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

/// The laboratory containers created in this state, reconstructed from each
/// container's `objectiveai.laboratory` label.
///
/// Runs `podman ps -a --filter name=objectiveai-laboratory-<state>- --format
/// json` (a JSON array of containers, each with a `Labels` object) and reads
/// the authoritative `objectiveai.laboratory` label per container (the label is
/// the round-trip record, avoiding podman's merged env/mount parsing).
/// Containers missing the label are skipped.
pub async fn list(ctx: &Context) -> Result<Vec<LaboratoryInfo>, Error> {
    let exe = ctx.podman().await?;
    let state = ctx.filesystem.state();
    let output = container_command(exe)
        .arg("ps")
        .arg("-a")
        .arg("--filter")
        .arg(format!("name=objectiveai-laboratory-{state}-"))
        .arg("--format")
        .arg("json")
        .output()
        .await
        .map_err(|e| Error::Podman(format!("spawn podman ps: {e}")))?;
    if !output.status.success() {
        return Err(Error::Podman(format!(
            "podman ps: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| Error::Podman(format!("parse podman ps output: {e}")))?;
    let array = value
        .as_array()
        .ok_or_else(|| Error::Podman("podman ps output: expected a JSON array".to_string()))?;
    let mut labs = Vec::new();
    for elem in array {
        let Some(label_str) = elem
            .get("Labels")
            .and_then(|l| l.get("objectiveai.laboratory"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let label: Label = serde_json::from_str(label_str)
            .map_err(|e| Error::Podman(format!("parse laboratory label: {e}")))?;
        labs.push(LaboratoryInfo {
            id: label.id,
            image: label.image,
            mounts: label
                .mounts
                .into_iter()
                .map(|m| Mount {
                    host: m.host,
                    container: m.container,
                })
                .collect(),
            env: label.env.into_iter().map(|[k, v]| (k, v)).collect(),
            cwd: label.cwd,
        });
    }
    Ok(labs)
}
