//! Internal laboratory-HOST spawn — the daemon starts the machine's
//! resident host as a leashed per-state child whenever a laboratories
//! flow needs it (there is no wire `laboratories spawn` command
//! anymore; `ensure_host`/`ensure_local_host` in the tier `mod.rs`
//! are the entries). The host is a WebSocket client (no listener), so
//! its stdout ready line carries no address — pure readiness.
//!
//! The dial list comes from config: unless `laboratories config local`
//! is false, the LOCAL daemon is ensured and dialed first (with the
//! signature from the DAEMON's own config — bare `SIGNATURE` env, else
//! derived from `SECRET`); then every `laboratories config addresses`
//! entry, each with its own optional signature. Everything rides argv
//! (`--address` repeated + `--signature ADDRESS=SIGNATURE` repeated) —
//! the host binary reads NO environment variables, by design.

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// How long a LOCAL spawn waits for the host to appear in the daemon
/// registry. Generous: podman (and its machine VM) may be cold.
const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// The spawn flow itself (used by the tier's `ensure_host` /
/// `ensure_local_host` auto-spawns). Idempotent and cheap when the
/// host is already up: a live resident child returns without
/// spawning. Returns every address the host was told to dial.
pub async fn spawn(global: &GlobalContext, scoped: &ScopedContext) -> Result<Vec<String>, Error> {
    let config = scoped
        .filesystem
        .read_config()
        .await?
        .laboratories
        .unwrap_or_default();
    let local = config.local != Some(false);

    let mut addresses: Vec<String> = Vec::new();
    let mut signatures: Vec<String> = Vec::new();
    if local {
        // Ensure the local daemon (idempotent) and dial it FIRST, with
        // the signature from the daemon's OWN config — never from the
        // addresses map.
        let daemon_address = crate::command::daemon::spawn::spawn(global, scoped).await?;
        if let Some(signature) = global.client_signature() {
            signatures.push(format!("{daemon_address}={signature}"));
        }
        addresses.push(daemon_address);
    }
    for (address, signature) in config.addresses.unwrap_or_default() {
        // The local daemon may also be a configured entry — one
        // connection per address, local signature wins.
        if addresses.contains(&address) {
            continue;
        }
        if !signature.is_empty() {
            signatures.push(format!("{address}={signature}"));
        }
        addresses.push(address);
    }
    if addresses.is_empty() {
        return Err(Error::Laboratory(
            "laboratories config local is false and no addresses are configured — \
             the host would have nothing to dial"
                .to_string(),
        ));
    }

    let exe = scoped.filesystem.bin_dir().join(if cfg!(windows) {
        "objectiveai-laboratory.exe"
    } else {
        "objectiveai-laboratory"
    });
    let _ = crate::spawn::spawn_leashed_until_ready(global, "laboratories", &exe, |cmd| {
        // No subcommand — the binary IS the host; bare args only.
        for address in &addresses {
            cmd.arg("--address").arg(address);
        }
        for signature in &signatures {
            cmd.arg("--signature").arg(signature);
        }
        cmd.arg("--objectiveai-dir")
            .arg(scoped.filesystem.dir())
            .arg("--objectiveai-state")
            .arg(scoped.filesystem.state())
            .arg("--suppress-output");
    })
    .await?;

    // Readiness. LOCAL: connected = this machine's host visible in the
    // daemon registry — poll it, failing fast if the leashed host
    // child dies. Remote-only: this machine cannot see the remote
    // registries; the stdout ready handshake is the whole contract
    // (the host retries its dials forever).
    if local {
        let machine_id =
            objectiveai_sdk::machine::machine_id(scoped.filesystem.dir());
        let deadline = std::time::Instant::now() + READY_TIMEOUT;
        loop {
            // Readiness = OUR host — the exact (machine, OWN state)
            // pair; a same-machine host of another state is somebody
            // else's.
            if let Some(hubs) = global.resident_hubs()
                && hubs
                    .laboratories
                    .has_host(&machine_id, scoped.filesystem.state())
            {
                break;
            }
            if !global.server_child_alive("laboratories") {
                return Err(Error::Laboratory(
                    "the laboratory host exited before connecting to the daemon"
                        .to_string(),
                ));
            }
            if std::time::Instant::now() >= deadline {
                return Err(Error::Laboratory(format!(
                    "the laboratory host did not connect to the daemon within {}s",
                    READY_TIMEOUT.as_secs()
                )));
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    }

    Ok(addresses)
}
