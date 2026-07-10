//! Laboratory Tauri commands: create / list / connect on the VIEWER's
//! machine — the #252 groundwork. Mirrors the CLI handlers: all podman
//! work happens inside the `objectiveai-laboratory` binary; the shell
//! only runs it as a subprocess.
//!
//! - `laboratories_create` runs `create` to completion (container +
//!   injected MCP binary, NOT started; hard error if the id exists).
//! - `laboratories_list` runs `list` to completion and returns the
//!   viewer machine's local scan. The daemon's merged view is already
//!   reachable from JS through the WebSocketExecutor, so this command
//!   adds exactly the capability JS cannot have.
//! - `laboratories_connect` takes NO arguments: it autonomously
//!   connects EVERY laboratory on this machine to the viewer's own
//!   daemon (address + signature from the managed
//!   [`crate::run::WebSocketConfig`]), spawning one resident manager
//!   per laboratory DETACHED via the SDK's lock-published discipline —
//!   idempotent per (id, address), so already-connected laboratories
//!   are no-ops. Readiness is lock publication (the viewer cannot
//!   reach a remote daemon's laboratories socket; managers retry
//!   their dials forever).
//!
//! NOT wired into the JS/UI yet.

use std::path::PathBuf;

use objectiveai_sdk::client_objectiveai_mcp::laboratory::{connect_lock_key, Identify};

/// The laboratory commands' environment: the layout root and state
/// name the subprocesses target. Managed by [`crate::run::serve`].
pub struct LabEnv {
    pub objectiveai_dir: PathBuf,
    pub state: String,
}

impl LabEnv {
    fn binary(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join(if cfg!(windows) {
            "objectiveai-laboratory.exe"
        } else {
            "objectiveai-laboratory"
        })
    }
}

#[derive(serde::Deserialize)]
pub struct Mount {
    pub host: String,
    pub container: String,
}

#[derive(serde::Deserialize)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

/// Create the laboratory container on this machine (waited; not
/// started; errors if the id already exists).
#[tauri::command]
pub(crate) async fn laboratories_create(
    env: tauri::State<'_, LabEnv>,
    id: String,
    image: String,
    mounts: Vec<Mount>,
    env_vars: Vec<EnvVar>,
    cwd: String,
) -> Result<(), String> {
    let mut cmd = tokio::process::Command::new(env.binary());
    cmd.arg("create")
        .arg("--id")
        .arg(&id)
        .arg("--image")
        .arg(&image)
        .arg("--cwd")
        .arg(&cwd)
        .arg("--objectiveai-dir")
        .arg(&env.objectiveai_dir)
        .arg("--objectiveai-state")
        .arg(&env.state);
    for mount in &mounts {
        cmd.arg("--mount")
            .arg(format!("{}:{}", mount.host, mount.container));
    }
    for var in &env_vars {
        cmd.arg("--env").arg(format!("{}={}", var.key, var.value));
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| format!("spawn objectiveai-laboratory create: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "objectiveai-laboratory create: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// The viewer machine's laboratories (running or not), from the local
/// podman label scan.
#[tauri::command]
pub(crate) async fn laboratories_list(
    env: tauri::State<'_, LabEnv>,
) -> Result<Vec<Identify>, String> {
    local_scan(&env).await
}

/// Run the manager binary's `list` and parse the identity array.
async fn local_scan(env: &LabEnv) -> Result<Vec<Identify>, String> {
    let output = tokio::process::Command::new(env.binary())
        .arg("list")
        .arg("--objectiveai-dir")
        .arg(&env.objectiveai_dir)
        .arg("--objectiveai-state")
        .arg(&env.state)
        .output()
        .await
        .map_err(|e| format!("spawn objectiveai-laboratory list: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "objectiveai-laboratory list: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("parse objectiveai-laboratory list output: {e}"))
}

/// Connect EVERY laboratory on this machine to the viewer's daemon.
/// No arguments: the target address + signature are the viewer's own
/// (the managed [`crate::run::WebSocketConfig`]), and the laboratory
/// set is the local scan. One detached manager per laboratory, all
/// spawned concurrently; idempotent per (id, address) via the
/// lock-publication discipline, so already-connected laboratories are
/// no-ops. Partial failures reject with every failure listed;
/// successes stay connected regardless. Resolves to nothing.
#[tauri::command]
pub(crate) async fn laboratories_connect(
    env: tauri::State<'_, LabEnv>,
    ws: tauri::State<'_, crate::run::WebSocketConfig>,
) -> Result<(), String> {
    let address = ws.address.clone();
    let signature = ws.signature.clone();
    let ids: Vec<String> = local_scan(&env)
        .await?
        .into_iter()
        .map(|lab| lab.id)
        .collect();

    let env_ref = &env;
    let results = futures::future::join_all(ids.into_iter().map(|id| {
        let address = address.clone();
        let signature = signature.clone();
        async move {
            connect_one(env_ref, &id, &address, signature.as_deref())
                .await
                .map_err(|e| format!("laboratory '{id}': {e}"))
        }
    }))
    .await;

    let failures: Vec<String> = results.into_iter().filter_map(Result::err).collect();
    if !failures.is_empty() {
        return Err(failures.join("; "));
    }
    Ok(())
}

/// Spawn (or find already-published) the detached manager for one
/// laboratory against `address`.
async fn connect_one(
    env: &LabEnv,
    id: &str,
    address: &str,
    signature: Option<&str>,
) -> Result<(), String> {
    let lock_dir = env
        .objectiveai_dir
        .join("state")
        .join(&env.state)
        .join("locks")
        .join("laboratories");
    let lock_key = connect_lock_key(id, address);

    objectiveai_sdk::lockfile::spawn_until_published(
        &env.binary(),
        &lock_dir,
        &lock_key,
        |cmd| {
            cmd.arg("connect")
                .arg("--id")
                .arg(id)
                .arg("--address")
                .arg(address)
                .arg("--objectiveai-dir")
                .arg(&env.objectiveai_dir)
                .arg("--objectiveai-state")
                .arg(&env.state)
                .arg("--suppress-output");
            // The signature travels by ENV VAR only; cleared when
            // absent so the child can't inherit a stale one.
            match signature {
                Some(s) => {
                    cmd.env("DAEMON_SIGNATURE", s);
                }
                None => {
                    cmd.env_remove("DAEMON_SIGNATURE");
                }
            }
        },
    )
    .await
    .map(|_published| ())
    .map_err(|e| e.to_string())
}
