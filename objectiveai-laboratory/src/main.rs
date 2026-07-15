//! `objectiveai-laboratory` — THE resident laboratory host for one
//! (machine, state).
//!
//! No subcommands: the binary IS the host, and the laboratory works
//! entirely over WebSocket. It holds the single `laboratories` lock in
//! `<state>/locks` (one host per state, however many daemon
//! connections it keeps), stops any containers a hard-killed
//! predecessor leaked, then dials `<address>/laboratory` for EVERY
//! `--address` — HostIdentify first (state + machine identity + the
//! full laboratory set), authorize second — serving MCP + transfer +
//! create/delete requests for ALL of the state's laboratories until
//! killed. Containers start lazily on their first routed op; on
//! graceful shutdown every container the host started is STOPPED
//! (never removed). All podman work (create/list/delete included)
//! happens in-process, driven by the daemons' forwarded requests.
//!
//! EVERYTHING is a clap argument — this binary reads NO environment
//! variables, by design: authorization signatures ride repeatable
//! `--signature ADDRESS=SIGNATURE` pairs alongside the repeatable
//! `--address` list.

mod channel;
mod cleaner;
mod filetree;
mod host;
mod podman;
mod server;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use objectiveai_sdk::laboratories::daemon::{Identify, IdentifyMount};

#[derive(Parser)]
#[command(name = "objectiveai-laboratory", version)]
struct Args {
    /// A daemon `http://` base address to connect to (its `/laboratory`
    /// route is appended). Repeatable — one resident connection per
    /// address.
    #[arg(long = "address")]
    addresses: Vec<String>,
    /// Authorization for one address: `ADDRESS=SIGNATURE`, split on
    /// the FIRST `=` (signatures themselves contain `=`). Repeatable.
    /// An address with no entry dials unauthenticated.
    #[arg(long = "signature")]
    signatures: Vec<String>,
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
        running: lab.running,
    }
}

#[tokio::main]
async fn main() {
    // No dotenv, no env reads — this binary's whole configuration is
    // its argv (see the module docs).
    let args = Args::parse();

    // Per-address signatures from the repeatable `--signature
    // ADDRESS=SIGNATURE` args, split on the FIRST `=` (the signature
    // itself contains one — `sha256=<hex>`).
    let signatures: std::collections::HashMap<String, String> = args
        .signatures
        .iter()
        .filter_map(|entry| {
            entry
                .split_once('=')
                .map(|(address, signature)| (address.to_string(), signature.to_string()))
        })
        .collect();

    // A host with nothing to dial is a caller bug (`laboratories
    // spawn` always passes at least one address) — fail loudly rather
    // than idle as an unreachable singleton.
    if args.addresses.is_empty() {
        eprintln!("no --address given — the host needs at least one daemon to serve");
        std::process::exit(1);
    }

    let objectiveai_dir = resolve_objectiveai_dir(&args.objectiveai_dir);
    let bin_dir = objectiveai_dir.join("bin");
    let lock_dir = objectiveai_dir
        .join("state")
        .join(&args.objectiveai_state)
        .join("locks");

    // ── THE single host lock ─────────────────────────────────────
    // One laboratory host per state, however many daemon connections
    // it keeps: claim key `laboratories` in `<state>/locks` — exactly
    // what the daemon spawner's `spawn_until_lock_published` probes.
    // The host is a WS client (no listener), so the content is a plain
    // readiness marker, not a URL. Simultaneous spawns race this
    // try_acquire and exactly one wins (the loser's spawner re-probes
    // the published lock — the api/mcp/viewer spawn discipline).
    let Some(claim) = objectiveai_sdk::lockfile::try_acquire(
        &lock_dir,
        "laboratories",
        "ready",
    )
    .await
    else {
        eprintln!(
            "another laboratory host already holds the `laboratories` lock for this state — exiting"
        );
        std::process::exit(1);
    };

    // ── Identity + the shared host server ────────────────────────
    let machine = objectiveai_sdk::machine::machine_identity(&objectiveai_dir);
    let server = Arc::new(host::HostServer::new(
        bin_dir.clone(),
        args.objectiveai_state.clone(),
        machine,
    ));

    // Stop leaked containers BEFORE serving: the host starts containers
    // strictly lazily, so nothing races this sweep until a channel is
    // up (see the cleaner's module docs).
    cleaner::sweep(bin_dir, args.objectiveai_state.clone()).await;

    // ── Serve every address until killed ─────────────────────────
    // One reconnect-forever channel per daemon address, all sharing
    // the one host server. On graceful shutdown every container the
    // host started is STOPPED (never removed): they and their
    // filesystems survive for the next host to `start` again. A hard
    // kill skips this — the next host's sweep stops them instead.
    let channels = futures::future::join_all(args.addresses.iter().map(|address| {
        channel::run(
            address.clone(),
            signatures.get(address).cloned(),
            Arc::clone(&server),
            args.suppress_output,
        )
    }));
    tokio::select! {
        _ = channels => {}
        _ = shutdown_signal() => {
            if !args.suppress_output {
                eprintln!("shutting down: stopping started laboratories");
            }
        }
    }
    server.stop_started().await;
    let _ = claim.release();
}

/// Resolves on a graceful-shutdown request: Ctrl+C everywhere, plus
/// SIGTERM on Unix (what `kill-all`'s Term attempt sends).
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
