//! Laboratory containers: resolve a laboratory id to its running podman
//! container and the host port its MCP server is published on.
//!
//! A laboratory's container is named `objectiveai-laboratory-<state>-<id>`
//! (`<state>` is the caller's namespace — the CLI passes its state name,
//! other hosts pass their own), so the same id in different
//! objectiveai states maps to different containers. Inside the container the
//! laboratory MCP server listens on the fixed port [`LAB_PORT`]; that port is
//! published to a random `127.0.0.1` host port the caller looks up here.
//!
//! [`create`] is the container's source of truth — `podman create` → `podman
//! cp` (inject the lab MCP binary) → `podman start`. [`host_port`] then resolves
//! the published host port for the conduit to dial.

use std::path::Path;

use super::Podman;
use super::Error;
use super::setup::MACHINE_NAME;

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
    objectiveai_sdk::process::no_window(&mut cmd);
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

/// Join a split
/// [`RegistryLaboratoryImage`](objectiveai_sdk::laboratories::RegistryLaboratoryImage)
/// into the reference string podman consumes — `registry/name:tag` or
/// `registry/name@digest`. THE only place the joined form exists: the
/// split shape is validated + fully qualified end to end, so podman
/// never gets a short name to silently resolve against docker.io.
fn registry_reference(
    image: &objectiveai_sdk::laboratories::RegistryLaboratoryImage,
) -> String {
    use objectiveai_sdk::laboratories::LaboratoryImagePin;
    match &image.pin {
        LaboratoryImagePin::Tag(tag) => {
            format!("{}/{}:{}", image.registry, image.name, tag)
        }
        LaboratoryImagePin::Digest(digest) => {
            format!("{}/{}@{}", image.registry, image.name, digest)
        }
    }
}

/// FNV-1a64 over `(state, id)` — the stable local tag for a lab's
/// inline-built image (`localhost/objectiveai-inline:<hex>`). Rebuilt
/// (and the tag overwritten) on every create.
fn inline_tag(state: &str, id: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in state.as_bytes().iter().chain([0u8].iter()).chain(id.as_bytes()) {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("localhost/objectiveai-inline:{hash:016x}")
}

/// Build an inline Containerfile into a locally-tagged image and
/// return the tag. The Containerfile is materialized into a scratch
/// dir that doubles as the (otherwise empty) build context — `COPY`
/// of local files fails by construction — and the dir is removed
/// afterwards. Runs on EVERY create; podman's layer cache does the
/// deduplication.
async fn build_inline(
    podman: &Podman,
    state: &str,
    id: &str,
    containerfile: &str,
) -> Result<String, Error> {
    let exe = podman.executable().await?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let scratch = podman
        .bin_dir()
        .join("inline-build")
        .join(format!("{nanos}"));
    tokio::fs::create_dir_all(&scratch)
        .await
        .map_err(|e| Error(format!("inline build scratch dir: {e}")))?;
    let containerfile_path = scratch.join("Containerfile");
    let write = tokio::fs::write(&containerfile_path, containerfile).await;
    if let Err(e) = write {
        let _ = tokio::fs::remove_dir_all(&scratch).await;
        return Err(Error(format!("write Containerfile: {e}")));
    }
    let tag = inline_tag(state, id);
    let output = container_command(&exe)
        .arg("build")
        .arg("-f")
        .arg(&containerfile_path)
        .arg("-t")
        .arg(&tag)
        .arg(&scratch)
        .output()
        .await;
    let _ = tokio::fs::remove_dir_all(&scratch).await;
    let output = output.map_err(|e| Error(format!("podman build: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman build failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(tag)
}

/// A laboratory container as read back by [`list`], reconstructed from its
/// `objectiveai.laboratory` label. Mirrors the `create` echo.
pub struct LaboratoryInfo {
    pub id: String,
    pub image: objectiveai_sdk::laboratories::LaboratoryImage,
    pub mounts: Vec<Mount>,
    pub env: Vec<(String, String)>,
    pub cwd: String,
    /// Unix seconds when the container was created, from podman's own
    /// container record (NOT the label). `None` when podman doesn't
    /// report it in a recognizable shape.
    pub created_at: Option<i64>,
    /// For agent laboratories: the full id of the agent the laboratory
    /// derives from. `None` for user-created laboratories.
    pub agent_full_id: Option<String>,
    /// Whether the container is RUNNING right now (podman's `State`),
    /// so consumers can distinguish a live laboratory from a created/
    /// stopped one — the lifecycle starts and stops containers on
    /// demand.
    pub running: bool,
}

/// The `objectiveai.laboratory` container label — the authoritative round-trip
/// record of how a laboratory was created, so `list` can reconstruct its spec
/// without relying on podman's merged env/mount parsing. Env is a list of
/// `[key, value]` pairs.
#[derive(serde::Serialize, serde::Deserialize)]
struct Label {
    id: String,
    image: objectiveai_sdk::laboratories::LaboratoryImage,
    mounts: Vec<LabelMount>,
    env: Vec<[String; 2]>,
    /// Default working directory for new agents. `#[serde(default)]` so
    /// containers created before this field round-trip in [`list`] as `/`.
    #[serde(default = "default_cwd")]
    cwd: String,
    /// For agent laboratories: the full id of the agent the laboratory
    /// derives from. Defaulted so pre-agent labels round-trip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_full_id: Option<String>,
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
    podman: &Podman,
    state: &str,
    machine_id: &str,
    laboratory_binary: &Path,
    id: &str,
    image: &objectiveai_sdk::laboratories::LaboratoryImage,
    mounts: &[Mount],
    env: &[(String, String)],
    cwd: &str,
    agent_full_id: Option<&str>,
) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let name = container_name(state, id);

    let label = Label {
        id: id.to_string(),
        image: image.clone(),
        mounts: mounts
            .iter()
            .map(|m| LabelMount {
                host: m.host.clone(),
                container: m.container.clone(),
            })
            .collect(),
        env: env.iter().map(|(k, v)| [k.clone(), v.clone()]).collect(),
        cwd: cwd.to_string(),
        agent_full_id: agent_full_id.map(str::to_string),
    };
    // Resolve what podman actually instantiates: a registry reference
    // joined here (the only place it exists), or the locally-built
    // tag of an inline Containerfile.
    let podman_image = match image {
        objectiveai_sdk::laboratories::LaboratoryImage::Registry(registry) => {
            registry_reference(registry)
        }
        objectiveai_sdk::laboratories::LaboratoryImage::Inline(inline) => {
            build_inline(podman, state, id, &inline.containerfile).await?
        }
    };
    let label_json = serde_json::to_string(&label)
        .map_err(|e| Error(format!("serialize laboratory label: {e}")))?;

    // 1. podman create
    let mut create_cmd = container_command(exe);
    create_cmd
        .arg("create")
        .arg("--name")
        .arg(&name)
        .arg("-p")
        .arg(format!("127.0.0.1::{LAB_PORT}/tcp"))
        // Grant CAP_SYS_ADMIN so the in-container MCP can run its
        // `fanotify` filesystem-change ATTRIBUTION watch (which agent
        // touched each file — surfaced by `/filetree`). There is no
        // narrower capability for fanotify; the kernel requires
        // CAP_SYS_ADMIN for filesystem/mount marks. Under rootless
        // podman this is NAMESPACE-scoped (admin over the container's
        // own user namespace, backed by an unprivileged host user —
        // NOT host root), and adding a capability never breaks an
        // image. Without it, `fanotify_init` fails EPERM and
        // attribution degrades silently (created_by/modified_by stay
        // absent). It is the single broadest capability, so it widens
        // the container's inner attack surface — the deliberate
        // trade-off for attribution.
        .arg("--cap-add")
        .arg("SYS_ADMIN");
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
    // The laboratory's COMPOSITE id `{machine}/{base62(state)}/{base62(id)}`
    // — the assistant-facing full identity (ids are only unique per
    // (machine, state)). The in-container MCP hashes it into its
    // `oail-<base62(fnv1a32(composite))>` server name (tool-name-safe
    // whatever the raw id looks like) and surfaces the composite
    // verbatim in its instructions. Baked into the container config at
    // create time → static and persists across restarts. Appended
    // after the user's env so it wins.
    let composite = objectiveai_sdk::laboratories::ClientLaboratory {
        r#type: objectiveai_sdk::laboratories::ClientLaboratoryType::Client,
        id: id.to_string(),
        machine: Some(machine_id.to_string()),
        machine_state: Some(state.to_string()),
    }
    .composite_id()
    .expect("machine + state are always present here");
    create_cmd
        .arg("-e")
        .arg(format!("OBJECTIVEAI_LABORATORY_ID={composite}"));
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
        .arg(podman_image);
    let output = create_cmd
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman create: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman create {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    // 2. podman cp — inject the caller-supplied musl MCP binary (the CLI
    // passes its staged `objectiveai-mcp-laboratory`; other hosts pass
    // their own copy — no `.exe`, ever).
    let output = container_command(exe)
        .arg("cp")
        .arg(laboratory_binary)
        .arg(format!("{name}:/objectiveai-mcp-laboratory"))
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman cp: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
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
pub async fn start(podman: &Podman, state: &str, id: &str) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let name = container_name(state, id);
    let output = container_command(exe)
        .arg("start")
        .arg(&name)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman start: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman start {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Stop a laboratory container (podman's default SIGTERM-then-SIGKILL
/// grace applies). STOP only — never remove: the container and its
/// filesystem survive for the next manager to `start` again. A
/// missing container reads as success (nothing to stop).
pub async fn stop(podman: &Podman, state: &str, id: &str) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let name = container_name(state, id);
    let output = container_command(exe)
        .arg("stop")
        .arg(&name)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman stop: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.to_ascii_lowercase().contains("no such container") {
            return Ok(());
        }
        return Err(Error(format!("podman stop {name}: {}", stderr.trim())));
    }
    Ok(())
}

/// The `127.0.0.1` host port the container's [`LAB_PORT`]/tcp is published on.
/// Force-remove the laboratory container (`podman rm -f`), reclaiming
/// its disk — removes it even if running. A missing container is
/// success (idempotent), matching [`stop`].
pub async fn remove(podman: &Podman, state: &str, id: &str) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let name = container_name(state, id);
    let output = container_command(exe)
        .arg("rm")
        .arg("-f")
        .arg(&name)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman rm: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.to_ascii_lowercase().contains("no such container") {
            return Ok(());
        }
        return Err(Error(format!("podman rm {name}: {}", stderr.trim())));
    }
    Ok(())
}

pub async fn host_port(podman: &Podman, state: &str, id: &str) -> Result<u16, Error> {
    let exe = podman.executable().await?;
    let name = container_name(state, id);
    let output = container_command(exe)
        .arg("port")
        .arg(&name)
        .arg(format!("{LAB_PORT}/tcp"))
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman port: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
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
            Error(format!("podman port {name}: no mapping for {LAB_PORT}/tcp"))
        })?;
    let port_str = line.rsplit_once(':').map(|(_, p)| p).unwrap_or(line);
    port_str.parse::<u16>().map_err(|e| {
        Error(format!(
            "podman port {name}: unparseable host port {port_str:?}: {e}"
        ))
    })
}

/// The ids of this state's laboratory containers that are RUNNING
/// right now (`podman ps` without `-a`), from the authoritative
/// `objectiveai.laboratory` label. The cleaner's candidate set —
/// stopped containers need no cleaning.
pub async fn list_running(podman: &Podman, state: &str) -> Result<Vec<String>, Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("ps")
        .arg("--filter")
        .arg(format!("name=objectiveai-laboratory-{state}-"))
        .arg("--filter")
        .arg("status=running")
        .arg("--format")
        .arg("json")
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman ps: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman ps: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| Error(format!("parse podman ps output: {e}")))?;
    let array = value
        .as_array()
        .ok_or_else(|| Error("podman ps output: expected a JSON array".to_string()))?;
    let mut ids = Vec::new();
    for elem in array {
        let Some(label_str) = elem
            .get("Labels")
            .and_then(|l| l.get("objectiveai.laboratory"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        // An unparseable label means the container is NOT ours — a
        // name/label coincidence from outside the objectiveai system
        // (or a pre-split-image relic). Treat it as nonexistent.
        let Ok(label) = serde_json::from_str::<Label>(label_str) else {
            continue;
        };
        ids.push(label.id);
    }
    Ok(ids)
}

/// The laboratory containers created in this state, reconstructed from each
/// container's `objectiveai.laboratory` label.
///
/// Runs `podman ps -a --filter name=objectiveai-laboratory-<state>- --format
/// json` (a JSON array of containers, each with a `Labels` object) and reads
/// the authoritative `objectiveai.laboratory` label per container (the label is
/// the round-trip record, avoiding podman's merged env/mount parsing).
/// Containers missing the label are skipped.
pub async fn list(podman: &Podman, state: &str) -> Result<Vec<LaboratoryInfo>, Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("ps")
        .arg("-a")
        .arg("--filter")
        .arg(format!("name=objectiveai-laboratory-{state}-"))
        .arg("--format")
        .arg("json")
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman ps: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman ps: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| Error(format!("parse podman ps output: {e}")))?;
    let array = value
        .as_array()
        .ok_or_else(|| Error("podman ps output: expected a JSON array".to_string()))?;
    let mut labs = Vec::new();
    for elem in array {
        let Some(label_str) = elem
            .get("Labels")
            .and_then(|l| l.get("objectiveai.laboratory"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        // Same rule as above: unparseable label ⇒ external container,
        // treat as nonexistent.
        let Ok(label) = serde_json::from_str::<Label>(label_str) else {
            continue;
        };
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
            created_at: created_at_from_container(elem),
            agent_full_id: label.agent_full_id,
            running: elem
                .get("State")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("running")),
        });
    }
    Ok(labs)
}

/// The container's creation time as unix seconds, tolerant of the two
/// shapes `podman ps --format json` has shipped for `Created`: an
/// integer of unix seconds, or an RFC3339 string. Anything else ⇒
/// `None` — the field is best-effort display metadata, never an error.
fn created_at_from_container(elem: &serde_json::Value) -> Option<i64> {
    let created = elem.get("Created")?;
    if let Some(secs) = created.as_i64() {
        return Some(secs);
    }
    created
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
}
