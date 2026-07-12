//! Laboratory Tauri commands: the machine-identity bridge and the
//! host spawner — everything else about laboratories reaches JS
//! through the daemon (the WebSocketExecutor and the
//! `/laboratories/*` streams), which is the ONLY laboratory data
//! path now that the `objectiveai-laboratory` binary is a pure
//! WebSocket host with no subcommands.
//!
//! - `machine_identity` returns THIS machine's
//!   [`objectiveai_sdk::machine::MachineIdentity`] — what JS compares
//!   against a laboratory's `machine.id` to classify "on this
//!   machine" (the old podman-scan membership test is gone with the
//!   scan itself; machine identity is the only provenance).
//! - `laboratories_spawn_host` starts THIS machine's resident
//!   laboratory HOST (one per (machine, state), serving ALL of its
//!   laboratories), dialing the viewer's own daemon (address +
//!   signature from the managed [`crate::run::WebSocketConfig`]).
//!   Idempotent via the single `laboratories` lock in
//!   `<state>/locks` — an already-running host is a no-op. Readiness
//!   is lock publication (the host retries its dial forever).

use std::path::PathBuf;

/// The laboratory commands' environment: the layout root and state
/// name the host subprocess targets. Managed by [`crate::run::serve`].
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

/// THIS machine's identity — the stable hashed id JS compares against
/// a laboratory's `machine.id`, plus the os/hostname display metadata.
#[tauri::command]
pub(crate) fn machine_identity(
    env: tauri::State<'_, LabEnv>,
) -> objectiveai_sdk::machine::MachineIdentity {
    objectiveai_sdk::machine::machine_identity(&env.objectiveai_dir)
}

/// Spawn (or find already-published) THIS machine's resident
/// laboratory HOST, dialing the viewer's own daemon. The target
/// address + signature are implicit — always the viewer's own (the
/// managed [`crate::run::WebSocketConfig`]). Idempotent via the single
/// `laboratories` lock: an already-running host is a no-op. Resolves
/// to nothing.
#[tauri::command]
pub(crate) async fn laboratories_spawn_host(
    env: tauri::State<'_, LabEnv>,
    ws: tauri::State<'_, crate::run::WebSocketConfig>,
) -> Result<(), String> {
    spawn_host(&env, &ws.address, ws.signature.as_deref())
        .await
        .map_err(|e| format!("laboratory host: {e}"))
}

/// The spawn itself: `--address <daemon>` under the single per-state
/// `laboratories` lock — no subcommand, the binary IS the host.
/// Everything rides argv — the host binary reads NO environment
/// variables, by design; the signature is the repeatable
/// `--signature ADDRESS=SIGNATURE` form.
async fn spawn_host(
    env: &LabEnv,
    address: &str,
    signature: Option<&str>,
) -> Result<(), String> {
    let lock_dir = env
        .objectiveai_dir
        .join("state")
        .join(&env.state)
        .join("locks");

    objectiveai_sdk::lockfile::spawn_until_published(
        &env.binary(),
        &lock_dir,
        "laboratories",
        |cmd| {
            cmd.arg("--address").arg(address);
            if let Some(signature) = signature {
                cmd.arg("--signature").arg(format!("{address}={signature}"));
            }
            cmd.arg("--objectiveai-dir")
                .arg(&env.objectiveai_dir)
                .arg("--objectiveai-state")
                .arg(&env.state)
                .arg("--suppress-output");
        },
    )
    .await
    .map(|_published| ())
    .map_err(|e| e.to_string())
}
