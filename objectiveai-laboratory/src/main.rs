//! `objectiveai-laboratory` — THE resident laboratory host for one
//! (machine, state).
//!
//! No subcommands: the binary IS the host, and the laboratory works
//! entirely over WebSocket. The daemon is its sole spawner and holds
//! its pipes; the host stops any containers a hard-killed predecessor
//! leaked, then serves MCP + transfer + create/delete requests for ALL
//! of the state's laboratories until killed.
//!
//! **The dial list rides STDIN, not argv — and it is DECLARATIVE.**
//! The host is born with ZERO daemon connections. The daemon writes
//! one [`objectiveai_sdk::laboratories::daemon::HostStdioRequest`]
//! JSON object per stdin line, each a `set_addresses` carrying the
//! ENTIRE desired dial list; the host CONVERGES to it — undesired
//! connections cancelled cooperatively (through the same detach path
//! a natural disconnect takes), new addresses dialed
//! (`<address>/laboratory`, HostIdentify first, authorize second),
//! changed-signature or dead-task entries re-dialed, identical live
//! ones left untouched — and answers with one ack line on stdout,
//! echoing the request's daemon-generated id for correlation. The
//! host never holds two connections to one address; an empty list
//! just idles. Stdin EOF means the daemon dropped the pipe (kill
//! under way / daemon death) — treated as a graceful-shutdown
//! request.
//!
//! Argv is layout-only (`--objectiveai-dir`, `--objectiveai-state`,
//! `--suppress-output`) — this binary reads NO environment variables,
//! by design.
//!
//! Containers start lazily on their first routed op; on graceful
//! shutdown every container the host started is STOPPED (never
//! removed).

mod channel;
mod cleaner;
mod ephemeral;
mod filetree;
mod host;
mod host_command;
mod lab_tree;
mod mount_watch;
mod plugin_image;
mod plugin_manifest;
mod podman;
mod server;
mod transfer;
mod upstream;
mod viewer_build;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use objectiveai_sdk::laboratories::daemon::{
    HostStdioAck, HostStdioCommand, Identify, IdentifyMount, IdentifyPlugin,
};

#[derive(Parser)]
#[command(name = "objectiveai-laboratory", version)]
struct Args {
    /// ObjectiveAI home; defaults to `~/.objectiveai`.
    #[arg(long)]
    objectiveai_dir: Option<PathBuf>,
    /// ObjectiveAI state name; defaults to `default`.
    #[arg(long, default_value = "default")]
    objectiveai_state: String,
    /// Suppress operational output.
    #[arg(long, default_value_t = false)]
    suppress_output: bool,
}

/// Windows-only: clear `HANDLE_FLAG_INHERIT` on this process's
/// stdin/stdout/stderr handles — which are the DAEMON'S pipes (the
/// stdin dial-list channel, the stdout ready/ack lane). Called ONCE at
/// startup, before any podman spawn: piped-stdio spawns set
/// `bInheritHandles=TRUE`, which copies EVERY inheritable handle into
/// the child — so without this, a podman child (worst case a long-lived
/// machine-VM helper) would hold the host→daemon pipe write ends past
/// the host's own death, and the daemon would never see pipe EOF (its
/// push death signal for this child, and its drain task's exit).
/// Best-effort, same contract as the daemon's `clear_stdio_inheritance`.
#[cfg(windows)]
fn clear_stdio_inheritance() {
    use std::os::windows::io::AsRawHandle;

    const HANDLE_FLAG_INHERIT: u32 = 0x1;
    // kernel32 is always linked on Windows targets; no #[link] needed.
    unsafe extern "system" {
        fn SetHandleInformation(
            h_object: *mut std::ffi::c_void,
            dw_mask: u32,
            dw_flags: u32,
        ) -> i32;
    }

    for handle in [
        std::io::stdin().as_raw_handle(),
        std::io::stdout().as_raw_handle(),
        std::io::stderr().as_raw_handle(),
    ] {
        if !handle.is_null() {
            unsafe {
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0);
            }
        }
    }
}

/// `<args.objectiveai_dir or ~/.objectiveai>` — the layout root.
fn resolve_objectiveai_dir(dir: &Option<PathBuf>) -> PathBuf {
    dir.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".objectiveai")
    })
}

/// [`podman::laboratory::LaboratoryInfo`] → its wire [`Identify`].
pub(crate) fn identify_from_info(lab: podman::laboratory::LaboratoryInfo) -> Identify {
    Identify {
        id: lab.id,
        image: lab.image,
        mounts: lab
            .mounts
            .into_iter()
            .map(|m| IdentifyMount {
                host: m.host,
                container: m.container,
            })
            .collect(),
        env: lab.env.into_iter().map(|(k, v)| [k, v]).collect(),
        cwd: lab.cwd,
        created_at: lab.created_at,
        agent_full_id: lab.agent_full_id,
        plugin: lab.plugin.map(|p| IdentifyPlugin {
            owner: p.owner,
            name: p.name,
            version: p.version,
        }),
        response_id: lab.response_id,
        running: lab.running,
    }
}

/// One live dial-list entry: the cancel handle for its
/// [`channel::run`] task, the task itself so convergence can await an
/// old connection's full teardown (channel detached) before re-dialing
/// its address, and the signature it was dialed with — what the
/// converge diff compares to decide identical-vs-re-dial.
struct Connection {
    cancel: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
    signature: Option<String>,
}

/// Cancel `connection` and wait for its channel task to finish its
/// teardown (detach from the host, close the socket).
async fn stop_connection(connection: Connection) {
    let _ = connection.cancel.send(true);
    let _ = connection.task.await;
}

/// Spawn one dial task: gate on the boot sweep (still answering a
/// cancel meanwhile), then [`channel::run`] — which retries its dial
/// forever until cancelled.
fn spawn_connection(
    server: &Arc<host::HostServer>,
    suppress_output: bool,
    swept: &tokio::sync::watch::Receiver<bool>,
    address: String,
    signature: Option<String>,
) -> Connection {
    let (cancel, cancel_rx) = tokio::sync::watch::channel(false);
    let task = tokio::spawn({
        let server = Arc::clone(server);
        let signature_task = signature.clone();
        let mut gate_cancel = cancel_rx.clone();
        let mut swept = swept.clone();
        async move {
            // Boot-sweep gate: the convergence is applied (and acked)
            // immediately, but dialing before the leaked-container
            // sweep finishes would break the cleaner invariant — wait
            // it out, still answering a cancel (a later converge).
            tokio::select! {
                _ = gate_cancel.changed() => return,
                _ = swept.wait_for(|done| *done) => {}
            }
            channel::run(address, signature_task, server, suppress_output, cancel_rx)
                .await
        }
    });
    Connection {
        cancel,
        task,
        signature,
    }
}

/// The stdin command loop — the host's whole dial-list authority. One
/// [`objectiveai_sdk::laboratories::daemon::HostStdioRequest`] JSON
/// object per line, each carrying the ENTIRE desired dial list
/// (`SetAddresses`); the host CONVERGES to it and acks on stdout with
/// the request's id echoed. Unparseable lines are ignored (nothing to
/// correlate an ack to). The `connections` map is keyed by address —
/// the uniqueness guarantee: the host never holds two connections to
/// one address (a changed entry tears the old connection down,
/// awaiting its full teardown, BEFORE dialing anew). Returns on stdin
/// EOF.
///
/// The converge diff, per desired entry:
/// - live connection, SAME signature, task still running → untouched
///   (no bounce — the whole point of the declarative protocol).
/// - live connection, different signature OR finished task (a panicked
///   dial task would otherwise stay dead forever) → stop + re-dial.
/// - no connection → dial.
///
/// Connections whose address is absent from the desired list are torn
/// down. An empty list is legal — the host idles.
async fn stdin_loop(
    server: Arc<host::HostServer>,
    suppress_output: bool,
    swept: tokio::sync::watch::Receiver<bool>,
) {
    use tokio::io::AsyncBufReadExt;
    let mut connections: std::collections::HashMap<String, Connection> =
        std::collections::HashMap::new();
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let Some(request) =
            objectiveai_sdk::laboratories::daemon::parse_host_stdio_request(&line)
        else {
            continue;
        };
        match request.command {
            HostStdioCommand::SetAddresses { addresses } => {
                // Desired set: first-seen order (dial order is
                // cosmetic — every connection dials independently),
                // last entry wins on duplicate addresses.
                let mut order: Vec<String> = Vec::new();
                let mut desired: std::collections::HashMap<String, Option<String>> =
                    std::collections::HashMap::new();
                for entry in addresses {
                    if !desired.contains_key(&entry.address) {
                        order.push(entry.address.clone());
                    }
                    desired.insert(entry.address, entry.signature);
                }
                // Tear down connections no longer desired.
                let stale: Vec<String> = connections
                    .keys()
                    .filter(|address| !desired.contains_key(*address))
                    .cloned()
                    .collect();
                for address in stale {
                    if let Some(old) = connections.remove(&address) {
                        stop_connection(old).await;
                    }
                }
                // Converge the desired entries.
                for address in order {
                    let signature = desired
                        .get(&address)
                        .cloned()
                        .expect("desired holds every ordered address");
                    let keep = connections.get(&address).is_some_and(|live| {
                        live.signature == signature && !live.task.is_finished()
                    });
                    if keep {
                        continue;
                    }
                    if let Some(old) = connections.remove(&address) {
                        stop_connection(old).await;
                    }
                    connections.insert(
                        address.clone(),
                        spawn_connection(
                            &server,
                            suppress_output,
                            &swept,
                            address,
                            signature,
                        ),
                    );
                }
            }
        }
        objectiveai_sdk::laboratories::daemon::print_host_stdio_ack(&HostStdioAck {
            id: request.id,
        });
    }
}

#[tokio::main]
async fn main() {
    // FIRST, before any podman spawn: keep the daemon's pipes out of
    // our children (see `clear_stdio_inheritance`).
    #[cfg(windows)]
    clear_stdio_inheritance();

    // No dotenv, no env reads — this binary's whole configuration is
    // its argv plus the stdin dial-list channel (see the module docs).
    let args = Args::parse();

    let objectiveai_dir = resolve_objectiveai_dir(&args.objectiveai_dir);
    let bin_dir = objectiveai_dir.join("bin");
    // ── Readiness handshake ──────────────────────────────────────
    // One laboratory host per (machine, state), enforced by the daemon
    // being its sole spawner (one leashed child per key — no
    // cross-process lock anymore). The host is a WS client (no
    // listener), so the ready line carries no address; the daemon
    // blocks on it before counting the host as up, then seeds the
    // dial list over stdin.
    objectiveai_sdk::process::print_ready(None);

    // ── Identity + the shared host server ────────────────────────
    let machine = objectiveai_sdk::machine::machine_identity(&objectiveai_dir);
    let server = Arc::new(host::HostServer::new(
        bin_dir.clone(),
        args.objectiveai_state.clone(),
        machine,
    ));

    // Stop leaked containers BEFORE serving: the host starts containers
    // strictly lazily, and every stdin-added dial task gates on this
    // barrier, so nothing races the sweep until a channel is up (see
    // the cleaner's module docs). The sweep runs CONCURRENTLY with the
    // stdin loop — podman may be cold (minutes), and the daemon's
    // dial-list acks must not wait on it.
    let (swept_tx, swept_rx) = tokio::sync::watch::channel(false);
    let sweep = {
        let bin_dir = bin_dir.clone();
        let state = args.objectiveai_state.clone();
        async move {
            cleaner::sweep(bin_dir, state).await;
            let _ = swept_tx.send(true);
            // Pend forever: this future rides a `join` with the
            // endless stdin loop, whose completion (EOF) is the arm's
            // real signal.
            std::future::pending::<()>().await;
        }
    };

    // ── Serve until killed ───────────────────────────────────────
    // The stdin loop owns the dial list — one reconnect-forever
    // channel task per added address, all sharing the one host
    // server. Zero addresses just idles (the loop keeps waiting on
    // stdin). On graceful shutdown — signal OR stdin EOF — every
    // container the host started is STOPPED (never removed): they and
    // their filesystems survive for the next host to `start` again. A
    // hard kill skips this — the next host's sweep stops them
    // instead. The channel tasks are left to die with the process;
    // their sockets drop with it.
    tokio::select! {
        _ = async {
            tokio::join!(sweep, stdin_loop(Arc::clone(&server), args.suppress_output, swept_rx))
        } => {
            if !args.suppress_output {
                eprintln!("stdin closed: stopping started laboratories");
            }
        }
        _ = shutdown_signal() => {
            if !args.suppress_output {
                eprintln!("shutting down: stopping started laboratories");
            }
        }
    }
    server.stop_started().await;
}

/// Resolves on a graceful-shutdown request: Ctrl+C everywhere, plus
/// SIGTERM on Unix (what the daemon's graceful kill sends).
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
