//! Laboratory-HOST spawn + dial-list CONVERGENCE — the daemon starts
//! the machine's resident host as a leashed per-state child. The host
//! must be spawned EXPLICITLY: `laboratories spawn` (its wire handler
//! is [`execute`] below) is the only entry; no flow auto-spawns it.
//! Like the other `spawn` commands it is a no-op when a host is
//! already running on this machine — a live resident child is reused,
//! not respawned. The host is a WebSocket client (no listener), so
//! its stdout ready line carries no address — pure readiness.
//!
//! The dial list is DECLARATIVE: config is the desired state, and
//! [`converge`] reconciles a LIVE host to it with one ack-gated
//! [`objectiveai_sdk::child_stdio::ChildStdioCommand::SetAddresses`]
//! (the host diffs — undesired connections torn down, new ones
//! dialed, identical live ones untouched). No host running means
//! nothing to reconcile: converge no-ops WITHOUT spawning or
//! blocking, and the next `spawn` converges from config. Every
//! `spawn` converges (fresh or reused — idempotent by construction),
//! and every `laboratories config` mutation converges after its
//! write; there is no seed/re-seed asymmetry and no compensating
//! revert choreography anymore.
//!
//! Desired entries: unless `laboratories config local` is false, the
//! LOCAL daemon is ensured and dialed first (with the signature from
//! the DAEMON's own config — bare `SIGNATURE` env, else derived from
//! `SECRET`); then every `laboratories config addresses` entry, each
//! with its own optional signature. Argv is layout-only
//! (`--objectiveai-dir` / `--objectiveai-state` /
//! `--suppress-output`); the host binary reads NO environment
//! variables, by design.

use objectiveai_sdk::cli::command::laboratories::spawn::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// The DESIRED dial list, straight from config: the local daemon
/// first (ensured — idempotent — with the daemon's own client
/// signature) unless `local` is false, then the configured addresses,
/// deduped (one connection per address; the local signature wins).
/// May legitimately be EMPTY (`local: false`, no addresses) — the
/// caller decides whether that's an error ([`spawn`]'s pre-flight) or
/// a valid "host idles" state ([`converge`]).
async fn desired_entries(
    global: &GlobalContext, scoped: &ScopedContext,
) -> Result<Vec<(String, Option<String>)>, Error> {
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
    Ok(entries)
}

/// Reconcile a LIVE host's dial list to config — the ONLY sender of
/// dial-list state. `Ok(None)` when there is nothing to reconcile: no
/// host running (converge NEVER spawns one — and never ensures the
/// daemon either, so a config command with no host has zero side
/// effects and returns immediately), or the host died mid-send (disk
/// already holds the desired state; the next `spawn` converges it).
/// `Ok(Some(addresses))` once the host acked the full list.
///
/// Serialized by its own gate ("laboratories/converge" — distinct
/// from the spawn gate, no nesting anywhere): the config is re-read
/// FRESH under the gate, so the last converge in gate order sends a
/// list at least as new as every write that preceded its lock — a
/// stale converge can never be the final word. A converge that errors
/// (or finds no host) leaves a live host at most temporarily behind
/// desired state; any later converge catches it up — self-healing,
/// no periodic reconciler.
pub(super) async fn converge(
    global: &GlobalContext, scoped: &ScopedContext,
) -> Result<Option<Vec<String>>, Error> {
    let gate = global.spawn_gate("laboratories/converge");
    let _guard = gate.lock().await;
    // Host check FIRST — before building entries — so a hostless
    // converge has no side effects (no local-daemon ensure).
    let Some(stdio) = global.resident_child_stdio("laboratories") else {
        return Ok(None);
    };
    let entries = desired_entries(global, scoped).await?;
    let command = objectiveai_sdk::child_stdio::ChildStdioCommand::SetAddresses {
        addresses: entries
            .iter()
            .map(|(address, signature)| {
                objectiveai_sdk::child_stdio::DialEntry {
                    address: address.clone(),
                    signature: signature.clone(),
                }
            })
            .collect(),
    };
    if stdio.send(&command).await.is_err() {
        // Broken channel = the host died between the liveness check
        // and the send. Not a command failure: config is the desired
        // state, and the next spawn converges from it.
        return Ok(None);
    }
    Ok(Some(
        entries.into_iter().map(|(address, _)| address).collect(),
    ))
}

/// The spawn flow itself. Idempotent and cheap when the host is
/// already up: a live resident child is reused, and the converge
/// below is a no-op diff host-side. Returns every address the host
/// dials.
pub async fn spawn(global: &GlobalContext, scoped: &ScopedContext) -> Result<Vec<String>, Error> {
    // Pre-flight on the CURRENT config: spawning a host with nothing
    // to dial is an error here (converge, by contrast, legitimately
    // sends an empty list — `del` of the last address idles the
    // host). Read once for the `local` readiness decision too.
    let config = scoped
        .filesystem
        .read_config()
        .await?
        .laboratories
        .unwrap_or_default();
    let local = config.local != Some(false);
    if !local && config.addresses.as_ref().is_none_or(|a| a.is_empty()) {
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
    crate::spawn::spawn_leashed_until_ready_with_stdio(
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

    // Converge the (fresh or reused) host to config — the one dial-
    // list mechanism. `None` here means the host we JUST ensured died
    // before it could be converged: error, don't wait on a dial-less
    // host. (It is NOT killed on a converge error — a resident host
    // with a stale/empty list self-heals on any later converge.)
    let Some(entries) = converge(global, scoped).await? else {
        return Err(Error::Laboratory(
            "the laboratory host exited before its dial list could be converged"
                .to_string(),
        ));
    };

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

    Ok(entries)
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
