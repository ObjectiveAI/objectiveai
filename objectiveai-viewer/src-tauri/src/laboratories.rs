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
//! - `laboratories_connect` spawns the resident manager DETACHED via
//!   the SDK's lock-published discipline, defaulting to the viewer's
//!   own daemon (address + signature from the managed
//!   [`crate::run::WebSocketConfig`]) — a local container serving a
//!   possibly-remote daemon, the viewer-side-laboratory story.
//!   Readiness is lock publication (the viewer cannot reach a remote
//!   daemon's laboratories socket; the manager retries its dial
//!   forever).
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

#[derive(serde::Serialize)]
pub struct Connected {
    pub id: String,
    pub address: String,
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

/// Connect a created laboratory on this machine to a daemon — the
/// viewer's own by default.
#[tauri::command]
pub(crate) async fn laboratories_connect(
    env: tauri::State<'_, LabEnv>,
    ws: tauri::State<'_, crate::run::WebSocketConfig>,
    id: String,
    address: Option<String>,
    signature: Option<String>,
) -> Result<Connected, String> {
    // Unset address = the viewer's daemon, with the viewer's own
    // signature; an explicit address takes the caller's signature (or
    // none — the remote daemon's signature is the caller's to supply).
    let (address, signature) = match address {
        Some(address) => (address, signature),
        None => (ws.address.clone(), ws.signature.clone()),
    };
    let lock_dir = env
        .objectiveai_dir
        .join("state")
        .join(&env.state)
        .join("locks")
        .join("laboratories");
    let lock_key = connect_lock_key(&id, &address);

    objectiveai_sdk::lockfile::spawn_until_published(
        &env.binary(),
        &lock_dir,
        &lock_key,
        |cmd| {
            cmd.arg("connect")
                .arg("--id")
                .arg(&id)
                .arg("--address")
                .arg(&address)
                .arg("--objectiveai-dir")
                .arg(&env.objectiveai_dir)
                .arg("--objectiveai-state")
                .arg(&env.state)
                .arg("--suppress-output");
            // The signature travels by ENV VAR only; cleared when
            // absent so the child can't inherit a stale one.
            match &signature {
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
    .map_err(|e| format!("connect laboratory '{id}': {e}"))?;

    Ok(Connected { id, address })
}
