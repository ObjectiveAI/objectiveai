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

/// Build an inline Containerfile into the given local `tag`. The
/// Containerfile is materialized into a scratch dir that doubles as
/// the (otherwise empty) build context — `COPY` of local files fails
/// by construction — and the dir is removed afterwards. Non-agent
/// creates run this EVERY time (podman's layer cache does the
/// deduplication); agent creates only when their stable tag is absent
/// (see [`create`]).
async fn build_inline(
    podman: &Podman,
    tag: &str,
    containerfile: &str,
) -> Result<(), Error> {
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
    let output = container_command(&exe)
        .arg("build")
        .arg("-f")
        .arg(&containerfile_path)
        .arg("-t")
        .arg(tag)
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
    Ok(())
}

/// The STABLE local image reference for an agent laboratory: the full
/// content-addressed lab id rides the TAG part (its base62 hash is
/// case-SENSITIVE — the reference's name path is lowercase-only, the
/// tag charset is not). Tag exists ⇒ the image is exactly what this
/// agent spec built/pulled before; skip the build/pull entirely.
fn agent_image_reference(id: &str) -> String {
    format!("localhost/objectiveai-agent:{id}")
}

/// Whether `reference` exists in local image storage. `podman image
/// exists` speaks in exit codes: 0 = present, 1 = absent (no output
/// either way); anything else is a real failure.
pub async fn image_exists(podman: &Podman, reference: &str) -> Result<bool, Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("image")
        .arg("exists")
        .arg(reference)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman image exists: {e}")))?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(Error(format!(
            "podman image exists {reference}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))),
    }
}

/// `podman pull` — fetch a registry reference into local storage.
pub async fn image_pull(podman: &Podman, reference: &str) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("pull")
        .arg(reference)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman pull: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman pull {reference}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// `podman tag` — attach `target` as another name for `source`'s image
/// ID. An alias, not a copy: the tag pins the exact image ID until it
/// is deliberately retagged, external images included.
pub async fn image_tag(podman: &Podman, source: &str, target: &str) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("tag")
        .arg(source)
        .arg(target)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman tag: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman tag {source} {target}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// `podman build -f <containerfile> -t <tag> [--label k=v …] <context>`
/// — the plugin-image build: the checkout root is the context, the
/// manifest's containerfile the build file, and the labels carry the
/// metadata the exists-fast-path reads back ([`image_label`]) without
/// re-cloning.
pub async fn image_build(
    podman: &Podman,
    containerfile: &Path,
    context: &Path,
    tag: &str,
    labels: &[(String, String)],
) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let mut cmd = container_command(exe);
    cmd.arg("build")
        .arg("-f")
        .arg(containerfile)
        .arg("-t")
        .arg(tag);
    for (key, value) in labels {
        cmd.arg("--label").arg(format!("{key}={value}"));
    }
    cmd.arg(context);
    let output = cmd
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman build: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman build {tag}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Read one label off a local image (`podman image inspect --format`).
/// `None` when the label is absent (or the image has no labels).
pub async fn image_label(
    podman: &Podman,
    reference: &str,
    key: &str,
) -> Result<Option<String>, Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("image")
        .arg("inspect")
        .arg("--format")
        .arg(format!("{{{{ index .Labels \"{key}\" }}}}"))
        .arg(reference)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman image inspect: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman image inspect {reference}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() || value == "<no value>" {
        return Ok(None);
    }
    Ok(Some(value))
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
    /// For plugin laboratories: the plugin's canonical coordinates plus
    /// the build metadata the host needs at start time (the container's
    /// internal MCP port). `None` for every other laboratory.
    pub plugin: Option<PluginLabel>,
    /// For EPHEMERAL laboratories: the agent-completion response id
    /// the laboratory serves. `None` for regular laboratories.
    pub response_id: Option<String>,
    /// Whether the container is RUNNING right now (podman's `State`),
    /// so consumers can distinguish a live laboratory from a created/
    /// stopped one — the lifecycle starts and stops containers on
    /// demand.
    pub running: bool,
}

/// A plugin laboratory's record inside the `objectiveai.laboratory`
/// label: the canonical coordinate trio (owner/name lowercased,
/// version case-preserved and `v`-prefixed), the container-internal
/// MCP port from the plugin manifest, and the git commit SHA the image
/// built from.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginLabel {
    pub owner: String,
    pub name: String,
    pub version: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
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
    /// For plugin laboratories: the plugin record. Defaulted so
    /// pre-plugin labels round-trip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    plugin: Option<PluginLabel>,
    /// For EPHEMERAL laboratories (agent and plugin): the
    /// agent-completion response id the laboratory serves. `Some` ⇔
    /// the container is ephemeral — the boot sweep REMOVES it instead
    /// of stopping it. Defaulted so pre-ephemeral labels round-trip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    response_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LabelMount {
    host: String,
    container: String,
}

/// Ensure the STABLE content-addressed agent image
/// (`localhost/objectiveai-agent:{derived_id}`) exists locally and
/// return its reference. Exists ⇒ use it verbatim — no build, no
/// pull. Missing ⇒ take the machine-wide bin lock, RE-CHECK (a
/// sibling host may have finished while we waited), then build
/// (Inline) or pull+tag (Registry — external images are tagged too,
/// pinning the exact image ID), releasing the lock the moment the
/// tag lands.
///
/// Keyed by the DERIVED (content-addressed) id — NOT the ephemeral
/// container id: every completion of the same (agent, spec) shares
/// one cached image while getting its own container.
pub async fn ensure_agent_image(
    podman: &Podman,
    derived_id: &str,
    image: &objectiveai_sdk::laboratories::LaboratoryImage,
) -> Result<String, Error> {
    let stable = agent_image_reference(derived_id);
    if image_exists(podman, &stable).await? {
        return Ok(stable);
    }
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &podman.bin_dir().join("locks"),
        &format!("agent-image-{derived_id}"),
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| Error(format!("bin lock: {e}")))?;
    let result = async {
        // Double-checked under the lock.
        if image_exists(podman, &stable).await? {
            return Ok(());
        }
        match image {
            objectiveai_sdk::laboratories::LaboratoryImage::Registry(registry) => {
                let joined = registry_reference(registry);
                image_pull(podman, &joined).await?;
                image_tag(podman, &joined, &stable).await
            }
            objectiveai_sdk::laboratories::LaboratoryImage::Inline(inline) => {
                build_inline(podman, &stable, &inline.containerfile).await
            }
        }
    }
    .await;
    // Release on EVERY path — a LockClaim drop deliberately does NOT
    // release (podman/install.rs pattern).
    claim
        .release()
        .map_err(|e| Error(format!("bin lock release: {e}")))?;
    result?;
    Ok(stable)
}

/// Create a REGULAR laboratory container (created, NOT started):
/// resolve the image (a registry reference joined here — the only
/// place the joined form exists — or an inline build under the
/// per-create FNV tag), then assemble the injected container.
/// Starting it is done elsewhere. Agent laboratories no longer pass
/// through here — they are EPHEMERAL, created via
/// [`create_agent_ephemeral`].
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
) -> Result<(), Error> {
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
        agent_full_id: None,
        plugin: None,
        response_id: None,
    };
    let podman_image = match image {
        objectiveai_sdk::laboratories::LaboratoryImage::Registry(registry) => {
            registry_reference(registry)
        }
        objectiveai_sdk::laboratories::LaboratoryImage::Inline(inline) => {
            let tag = inline_tag(state, id);
            build_inline(podman, &tag, &inline.containerfile).await?;
            tag
        }
    };
    create_injected_container(
        podman,
        state,
        machine_id,
        laboratory_binary,
        id,
        &podman_image,
        mounts,
        env,
        // Regular laboratories are multi-session/multi-identity —
        // identity rides per-request headers, never container env.
        &[],
        cwd,
        &label,
    )
    .await
}

/// Create an EPHEMERAL agent-laboratory container (created, NOT
/// started — the ephemeral flow starts it immediately after). The
/// container id embeds the completion's response id; `resolved_image`
/// is the pre-ensured stable agent tag ([`ensure_agent_image`]); the
/// label records the WIRE image spec plus the agent provenance AND
/// the response id (`Some` ⇔ ephemeral — the boot sweep removes it).
/// Agent laboratories have no mounts.
#[allow(clippy::too_many_arguments)]
pub async fn create_agent_ephemeral(
    podman: &Podman,
    state: &str,
    machine_id: &str,
    laboratory_binary: &Path,
    id: &str,
    image: &objectiveai_sdk::laboratories::LaboratoryImage,
    resolved_image: &str,
    env: &[(String, String)],
    identity_env: &[(String, String)],
    cwd: &str,
    agent_full_id: &str,
    response_id: &str,
) -> Result<(), Error> {
    let label = Label {
        id: id.to_string(),
        image: image.clone(),
        mounts: Vec::new(),
        env: env.iter().map(|(k, v)| [k.clone(), v.clone()]).collect(),
        cwd: cwd.to_string(),
        agent_full_id: Some(agent_full_id.to_string()),
        plugin: None,
        response_id: Some(response_id.to_string()),
    };
    create_injected_container(
        podman,
        state,
        machine_id,
        laboratory_binary,
        id,
        resolved_image,
        &[],
        env,
        identity_env,
        cwd,
        &label,
    )
    .await
}

/// The shared assembly core for INJECTED-MCP containers (regular and
/// ephemeral-agent laboratories): `podman create` — [`LAB_PORT`]
/// publish, `-v` mounts, user env, then the host-owned stamps
/// (`PORT`, `OBJECTIVEAI_LABORATORY_ID`, `OBJECTIVEAI_LABORATORY_CWD`,
/// `OBJECTIVEAI_FILETREE_IGNORE`), the `objectiveai.laboratory`
/// label, and the chmod+exec entrypoint wrapper — followed by
/// `podman cp` of the `objectiveai-mcp-laboratory` musl binary.
async fn create_injected_container(
    podman: &Podman,
    state: &str,
    machine_id: &str,
    laboratory_binary: &Path,
    id: &str,
    podman_image: &str,
    mounts: &[Mount],
    env: &[(String, String)],
    identity_env: &[(String, String)],
    cwd: &str,
    label: &Label,
) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let name = container_name(state, id);
    let label_json = serde_json::to_string(label)
        .map_err(|e| Error(format!("serialize laboratory label: {e}")))?;

    // 1. podman create
    let mut create_cmd = container_command(exe);
    create_cmd
        .arg("create")
        .arg("--name")
        .arg(&name)
        .arg("-p")
        .arg(format!("127.0.0.1::{LAB_PORT}/tcp"));
    // NO --cap-add, deliberately: laboratories must run with the
    // default (unprivileged) capability set so the same image runs on
    // managed-cloud platforms — Kubernetes baseline/restricted Pod
    // Security, Fargate, Cloud Run all refuse capability grants.
    // Nothing in-container needs privilege: the filetree watch is
    // inotify (unprivileged everywhere). A CAP_SYS_ADMIN grant used to
    // live here for fanotify attribution, but probes proved fanotify
    // can never arm in these containers (rootless kernel policy
    // ignores namespaced caps; overlay/9p lack the exportfs support
    // FID marks need) — see
    // objectiveai-mcp-laboratory/src/attribution.rs.
    for m in mounts {
        create_cmd
            .arg("-v")
            .arg(format!("{}:{}", m.host, m.container));
    }
    for (k, v) in env {
        create_cmd.arg("-e").arg(format!("{k}={v}"));
    }
    // The AGENT-IDENTITY environment (ephemeral laboratories only —
    // one completion per container, so the identity is static for the
    // container's whole life). Appended AFTER the user's env so it
    // wins; the SDK's agent-laboratory validate additionally rejects
    // user declarations of these reserved names outright.
    for (k, v) in identity_env {
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
    // The `/filetree` ignore set — HOST-OWNED, exclusively: the
    // in-container MCP is deliberately naive and just hides whatever
    // paths this env lists, and this stamp is the only writer (it is
    // appended after the user's env, so podman's last-wins semantics
    // make any user-supplied value inert). The mount concept lives
    // HERE, so this is where the policy is decided:
    //
    // - Every mount's container path: filesystem mounts (9p/virtiofs)
    //   deliver ZERO inotify events (proven empirically — the mount
    //   protocol has no fsnotify path) while walking them is ~25×
    //   slower than native, so showing them would mean a minutes-long
    //   frozen snapshot that then never updates. Mounted host folders
    //   are THIS host's to watch natively, later.
    // - The kernel pseudo-filesystems podman mounts into every
    //   container: they churn constantly, aren't laboratory data, and
    //   their magic files abort inotify registration wholesale
    //   (`watch /` used to die on `/proc/tty/driver` with EACCES).
    let filetree_ignore = ["/proc", "/sys", "/dev"]
        .into_iter()
        .chain(mounts.iter().map(|m| m.container.as_str()))
        .collect::<Vec<_>>()
        .join(":");
    create_cmd
        .arg("-e")
        .arg(format!("OBJECTIVEAI_FILETREE_IGNORE={filetree_ignore}"));
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

/// Create an EPHEMERAL PLUGIN laboratory container (created, NOT
/// started — the ephemeral flow starts it immediately after).
///
/// Deliberately minimal next to [`create`]: the image's OWN entrypoint
/// runs the plugin's MCP server — NO `objectiveai-mcp-laboratory`
/// injection, NO `--entrypoint` override, and no env beyond the
/// AGENT-IDENTITY set (the author declared the listen port in the
/// plugin manifest, so there is nothing to force). The manifest port is published to a random loopback host
/// port ([`host_port`] resolves it with `plugin.port` as the internal
/// port), and the `objectiveai.laboratory` label records the localhost
/// image reference, the [`PluginLabel`], and the completion's
/// response id (`Some` ⇔ ephemeral — the boot sweep removes it).
pub async fn create_plugin(
    podman: &Podman,
    state: &str,
    id: &str,
    image: &objectiveai_sdk::laboratories::RegistryLaboratoryImage,
    plugin: &PluginLabel,
    response_id: &str,
    identity_env: &[(String, String)],
) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let name = container_name(state, id);
    let label = Label {
        id: id.to_string(),
        image: objectiveai_sdk::laboratories::LaboratoryImage::Registry(image.clone()),
        mounts: Vec::new(),
        env: Vec::new(),
        // Display-only for plugin labs: the image's own WORKDIR
        // governs where the entrypoint runs.
        cwd: default_cwd(),
        agent_full_id: None,
        plugin: Some(plugin.clone()),
        response_id: Some(response_id.to_string()),
    };
    let label_json = serde_json::to_string(&label)
        .map_err(|e| Error(format!("serialize laboratory label: {e}")))?;
    let mut create_cmd = container_command(exe);
    create_cmd
        .arg("create")
        .arg("--name")
        .arg(&name)
        .arg("-p")
        .arg(format!("127.0.0.1::{}/tcp", plugin.port))
        // Make `host.containers.internal` resolve to the host so the
        // plugin can reach this host's Postgres tunnel listener
        // (`OBJECTIVEAI_POSTGRES_URL`).
        .arg("--add-host")
        .arg("host.containers.internal:host-gateway");
    // The AGENT-IDENTITY environment (plus `OBJECTIVEAI_POSTGRES_URL`)
    // — the env a plugin container gets (the completion it serves is
    // static for the container's whole life; everything else the
    // plugin needs rides headers).
    for (k, v) in identity_env {
        create_cmd.arg("-e").arg(format!("{k}={v}"));
    }
    let output = create_cmd
        .arg("--label")
        .arg(format!("objectiveai.laboratory={label_json}"))
        .arg(registry_reference(image))
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman create: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman create {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Create a container for EXPORT ONLY — never started. The
/// viewer-plugin build bakes its artifact into the image with `RUN`
/// steps, so all that remains is to open a filesystem view of it,
/// copy the artifact out ([`copy_out`]), and remove both. No label
/// (this is not a laboratory), no port, no mounts, no injection.
///
/// The trailing placeholder command is what makes this work for ANY
/// image: podman refuses to create a container from an image with
/// neither `CMD` nor `ENTRYPOINT` (a `FROM scratch` final stage, say),
/// and since the container is never started, what the command says is
/// irrelevant — only that one exists.
pub async fn create_for_export(
    podman: &Podman,
    name: &str,
    image_reference: &str,
) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("create")
        .arg("--name")
        .arg(name)
        .arg(image_reference)
        .arg("/objectiveai-never-runs")
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman create: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman create {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Copy a directory's CONTENTS out of a container (`podman cp
/// <name>:<path>/. <destination>` — the trailing `/.` is what copies
/// the contents rather than the directory itself). The destination
/// must already exist.
pub async fn copy_out(
    podman: &Podman,
    name: &str,
    container_path: &str,
    destination: &Path,
) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let source = format!("{name}:{}/.", container_path.trim_end_matches('/'));
    let output = container_command(exe)
        .arg("cp")
        .arg(&source)
        .arg(destination)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman cp: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman cp {source}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Remove a container by its RAW name — the export container, which
/// is not a laboratory and so has no `(state, id)` to derive one from.
pub async fn remove_named(podman: &Podman, name: &str) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("rm")
        .arg("-f")
        .arg(name)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman rm: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman rm {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// `podman rmi -f` — drop a local image tag. The viewer build's image
/// exists only to be copied out of, so it is removed the moment the
/// copy lands: the layers this build ADDED are freed, while the
/// plugin's base image stays tagged and shared (repeat builds still
/// skip the pull).
pub async fn image_remove(podman: &Podman, reference: &str) -> Result<(), Error> {
    let exe = podman.executable().await?;
    let output = container_command(exe)
        .arg("rmi")
        .arg("-f")
        .arg(reference)
        .output()
        .await
        .map_err(|e| Error(format!("spawn podman rmi: {e}")))?;
    if !output.status.success() {
        return Err(Error(format!(
            "podman rmi {reference}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
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

pub async fn host_port(
    podman: &Podman,
    state: &str,
    id: &str,
    internal_port: u16,
) -> Result<u16, Error> {
    let exe = podman.executable().await?;
    let name = container_name(state, id);
    let output = container_command(exe)
        .arg("port")
        .arg(&name)
        .arg(format!("{internal_port}/tcp"))
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
            Error(format!("podman port {name}: no mapping for {internal_port}/tcp"))
        })?;
    let port_str = line.rsplit_once(':').map(|(_, p)| p).unwrap_or(line);
    port_str.parse::<u16>().map_err(|e| {
        Error(format!(
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
            plugin: label.plugin,
            response_id: label.response_id,
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
