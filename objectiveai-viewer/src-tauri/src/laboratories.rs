//! Laboratory Tauri commands: the machine-identity bridge —
//! everything else about laboratories reaches JS through the daemon
//! (the SseCommandExecutor and the `/laboratories/*` streams), and
//! the laboratory HOST is spawned exclusively by the daemon as one of
//! its leashed resident children (the viewer's former direct
//! host-spawn path is gone with the server lockfiles).
//!
//! `machine_identity` returns THIS machine's
//! [`objectiveai_sdk::machine::MachineIdentity`] — what JS compares
//! against a laboratory's `machine.id` to classify "on this machine"
//! (machine identity is the only provenance).

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
