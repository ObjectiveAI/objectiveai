//! Internal laboratory-HOST spawn — the daemon starts the machine's
//! resident host as a leashed per-state child whenever a laboratories
//! flow needs it (there is no wire `laboratories spawn` command
//! anymore; `ensure_host`/`ensure_local_host` in the tier `mod.rs`
//! are the entries). The host is a WebSocket client (no listener), so
//! its stdout ready line carries no address — pure readiness.
//!
//! The dial list comes from config but rides STDIN, not argv: unless
//! `laboratories config local` is false, the LOCAL daemon is ensured
//! and dialed first (with the signature from the DAEMON's own config —
//! bare `SIGNATURE` env, else derived from `SECRET`); then every
//! `laboratories config addresses` entry, each with its own optional
//! signature. A FRESHLY spawned host is seeded with one ack-gated
//! [`objectiveai_sdk::laboratories::daemon::HostStdioCommand::AddAddress`]
//! per entry right after its ready line; a reused live child is NOT
//! re-seeded — config changes reach it through the `laboratories
//! config` handlers' stdio sends. Argv is layout-only
//! (`--objectiveai-dir` / `--objectiveai-state` /
//! `--suppress-output`); the host binary reads NO environment
//! variables, by design.

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

    let mut entries: Vec<(String, Option<String>)> = Vec::new();
    if local {
        // Ensure the local daemon (idempotent) and dial it FIRST, with
        // the signature from the daemon's OWN config — never from the
        // addresses map.
        let daemon_address = crate::command::daemon::spawn::spawn(global, scoped).await?;
        entries.push((daemon_address, global.client_signature()));
    }
    for (address, signature) in config.addresses.unwrap_or_default() {
        // The local daemon may also be a configured entry — one
        // connection per address, local signature wins.
        if entries.iter().any(|(a, _)| a == &address) {
            continue;
        }
        let signature = (!signature.is_empty()).then_some(signature);
        entries.push((address, signature));
    }
    if entries.is_empty() {
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
    let (_, freshly_spawned) = crate::spawn::spawn_leashed_until_ready_with_stdio(
        global,
        "laboratories",
        &exe,
        |cmd| {
            // No subcommand — the binary IS the host; layout args
            // only (the dial list rides stdin below).
            cmd.arg("--objectiveai-dir")
                .arg(scoped.filesystem.dir())
                .arg("--objectiveai-state")
                .arg(scoped.filesystem.state())
                .arg("--suppress-output");
        },
    )
    .await?;

    // Seed the fresh host's dial list, ack-gated per address. A host
    // that cannot accept its seed list is broken — propagate.
    if freshly_spawned {
        let stdio = global.lab_host_stdio().ok_or_else(|| {
            Error::Laboratory(
                "the laboratory host exited before its dial list could be seeded"
                    .to_string(),
            )
        })?;
        for (address, signature) in &entries {
            stdio
                .send_host_stdio(
                    &objectiveai_sdk::laboratories::daemon::HostStdioCommand::AddAddress {
                        address: address.clone(),
                        signature: signature.clone(),
                    },
                )
                .await?;
        }
    }

    // Readiness. LOCAL: connected = this machine's host visible in the
    // daemon registry — poll it, failing fast if the leashed host
    // child dies. Remote-only: this machine cannot see the remote
    // registries; the stdout ready handshake plus the acked seed is
    // the whole contract (the host retries its dials forever).
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

    Ok(entries.into_iter().map(|(address, _)| address).collect())
}
