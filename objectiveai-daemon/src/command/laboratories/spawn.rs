//! Laboratory-HOST spawn — the daemon starts the machine's resident
//! host as a leashed per-state child. The host must be spawned
//! EXPLICITLY: `laboratories spawn` (its wire handler is [`execute`]
//! below) is the only entry; no flow auto-spawns it. Like the other
//! `spawn` commands it is a no-op when a host is already running on
//! this machine — a live resident child is reused, not respawned. The
//! host is a WebSocket client (no listener), so its stdout ready line
//! carries no address — pure readiness.
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

use objectiveai_sdk::cli::command::laboratories::spawn::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// The spawn flow itself. Idempotent and cheap when the host is already
/// up: a live resident child returns without spawning. Returns every
/// address the host was told to dial.
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
    // that cannot accept its seed list is broken — propagate, but
    // RETIRE the child first: it was parked as the resident host
    // before seeding, and leaving it would make every later ensure
    // reuse a dial-less host (fresh children are the only ones
    // seeded).
    if freshly_spawned {
        let seed = async {
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
            Ok::<(), Error>(())
        };
        if let Err(e) = seed.await {
            let _ = crate::command::kill_helpers::kill_resident_child(global, "laboratories")
                .await;
            return Err(e);
        }
    }

    // Readiness. LOCAL: connected = this machine's host visible in the
    // daemon registry. The daemon is the WS SERVER for the host's
    // `/laboratory` connection, so it OBSERVES the registration directly
    // — we AWAIT the registry's `HostConnected` broadcast, never poll.
    // No deadline: a cold podman machine can legitimately take minutes
    // to enumerate for its HostIdentify, and we don't cap that. The one
    // failure we watch is the leashed host child dying before it
    // connects — a PUSH watch, not a `try_wait` poll. Remote-only: this
    // machine cannot see the remote registries; the stdout ready
    // handshake plus the acked seed is the whole contract (the host
    // retries its dials forever).
    if local {
        let machine_id =
            objectiveai_sdk::machine::machine_id(scoped.filesystem.dir());
        let state = scoped.filesystem.state();
        let hubs = global.resident_hubs().ok_or_else(|| {
            Error::Laboratory("laboratories spawn requires the resident daemon".to_string())
        })?;
        // Subscribe BEFORE the first check: a registration racing in
        // after this point lands as a buffered event — none is lost.
        let mut changes = hubs.laboratories.subscribe();
        // PUSH death watch of the freshly-spawned child (fired by its
        // drain task on pipe EOF); `None`/already-`true` = already gone.
        let mut dead = global.resident_child_dead_rx("laboratories");
        loop {
            // Readiness = OUR host — the exact (machine, OWN state)
            // pair; a same-machine host of another state is somebody
            // else's. Checked FIRST every wake, so a host that
            // registers and then dies still counts as up.
            if hubs.laboratories.has_host(&machine_id, state) {
                break;
            }
            let already_dead = match dead.as_ref() {
                Some(rx) => *rx.borrow(),
                None => true,
            };
            if already_dead {
                return Err(Error::Laboratory(
                    "the laboratory host exited before connecting to the daemon"
                        .to_string(),
                ));
            }
            tokio::select! {
                // A connected-set change (or a lagged feed) → re-check
                // has_host at the top. A CLOSED feed = the registry is
                // gone (daemon teardown); stop waiting.
                recv = changes.recv() => {
                    if matches!(
                        recv,
                        Err(tokio::sync::broadcast::error::RecvError::Closed)
                    ) {
                        return Err(Error::Laboratory(
                            "the daemon registry closed before the host connected"
                                .to_string(),
                        ));
                    }
                }
                // The child died (watch fired) OR its watch CLOSED
                // without firing (drain task gone — the observer is
                // dead, so nothing would ever fire; treating it as
                // alive would busy-spin on the closed channel). Either
                // way: drop the watch so the top of the loop errors
                // unless a registration also landed this wake.
                _ = async {
                    match dead.as_mut() {
                        Some(rx) => {
                            let _ = rx.changed().await;
                        }
                        None => std::future::pending().await,
                    }
                } => {
                    let fired = dead
                        .as_ref()
                        .is_some_and(|rx| *rx.borrow());
                    if !fired {
                        // Closed-without-firing: no death signal will
                        // ever come. `None` reads as dead at the top.
                        dead = None;
                    }
                }
            }
        }
    }

    Ok(entries.into_iter().map(|(address, _)| address).collect())
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    Ok(Response {
        addresses: spawn(global, scoped).await?,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::spawn as sdk;
    use objectiveai_sdk::cli::command::laboratories::spawn::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::spawn as sdk;
    use objectiveai_sdk::cli::command::laboratories::spawn::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
