//! `objectiveai-laboratory` — the laboratory host binary.
//!
//! Subcommands:
//!
//! - **`host`** — THE resident laboratory host for this (machine,
//!   state): hold the single `laboratories` lock in `<state>/locks`
//!   (one host per state, however many daemon connections it keeps),
//!   stop any containers a hard-killed predecessor leaked, then dial
//!   `<address>/laboratory` for EVERY `--address` — HostIdentify
//!   first (state + machine identity + the full laboratory set),
//!   authorize second — serving MCP + transfer + create/delete
//!   requests for ALL of the state's laboratories until killed.
//!   Containers start lazily on their first routed op; on graceful
//!   shutdown every container the host started is STOPPED (never
//!   removed).
//! - **`create`** — create the laboratory container (podman create +
//!   inject the MCP binary; NOT started) and exit. Errors if the id
//!   already exists in this state. Pure and daemonless: no lock, no
//!   WebSocket, no signature. (Daemons don't shell to this anymore —
//!   they forward `LaboratoryCreate` over the host's WS — but it
//!   remains as local tooling.)
//! - **`list`** — print the state's laboratory containers (running or
//!   not) as a JSON array of identity objects on stdout.
//! - **`delete`** — force-remove the laboratory container and exit.
//!
//! Everything is a clap argument except the authorization signatures,
//! which arrive via the `DAEMON_SIGNATURES` environment variable — a
//! JSON map of daemon address → signature, holding only the addresses
//! that have one — so they never appear in a process listing.

mod channel;
mod cleaner;
mod host;
mod podman;
mod server;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{Identify, IdentifyMount};

#[derive(Parser)]
#[command(name = "objectiveai-laboratory", version)]
enum Command {
    /// Create the laboratory container (not started); errors if the id
    /// already exists in this state.
    Create(CreateArgs),
    /// Run the resident laboratory host: serve ALL of this state's
    /// laboratories to every given daemon address (until killed).
    Host(HostArgs),
    /// Print the state's laboratory containers (running or not) as a
    /// JSON array of identity objects on stdout.
    List(ListArgs),
    /// Force-remove the laboratory container (`podman rm -f`),
    /// reclaiming disk. A missing container is not an error.
    Delete(DeleteArgs),
}

#[derive(clap::Args)]
struct CreateArgs {
    /// The laboratory id (unique within the state).
    #[arg(long)]
    id: String,
    /// Container image for the laboratory.
    #[arg(long)]
    image: String,
    /// Bind mount, `host:container`. Repeatable. The LAST `:` splits
    /// (host paths may be Windows drive paths).
    #[arg(long = "mount")]
    mounts: Vec<String>,
    /// Environment variable for the container, `KEY=VALUE`. Repeatable.
    #[arg(long = "env")]
    env: Vec<String>,
    /// Default working directory new agents start in.
    #[arg(long)]
    cwd: String,
    /// ObjectiveAI home; defaults to `~/.objectiveai`.
    #[arg(long)]
    objectiveai_dir: Option<PathBuf>,
    /// ObjectiveAI state name; defaults to `default`.
    #[arg(long, default_value = "default")]
    objectiveai_state: String,
}

#[derive(clap::Args)]
struct HostArgs {
    /// A daemon `ws://` base address to connect to (its `/laboratory`
    /// route is appended). Repeatable — one resident connection per
    /// address.
    #[arg(long = "address")]
    addresses: Vec<String>,
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

#[derive(clap::Args)]
struct ListArgs {
    /// ObjectiveAI home; defaults to `~/.objectiveai`.
    #[arg(long)]
    objectiveai_dir: Option<PathBuf>,
    /// ObjectiveAI state name; defaults to `default`.
    #[arg(long, default_value = "default")]
    objectiveai_state: String,
}

#[derive(clap::Args)]
struct DeleteArgs {
    /// The laboratory id to remove.
    #[arg(long)]
    id: String,
    /// ObjectiveAI home; defaults to `~/.objectiveai`.
    #[arg(long)]
    objectiveai_dir: Option<PathBuf>,
    /// ObjectiveAI state name; defaults to `default`.
    #[arg(long, default_value = "default")]
    objectiveai_state: String,
}

/// `<args.objectiveai_dir or ~/.objectiveai>` — the layout root.
fn resolve_objectiveai_dir(dir: &Option<PathBuf>) -> PathBuf {
    dir.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".objectiveai")
    })
}

#[tokio::main]
async fn main() {
    // Load a `.env` if present BEFORE reading DAEMON_SIGNATURES.
    let _ = dotenv::dotenv();
    match Command::parse() {
        Command::Create(args) => create(args).await,
        Command::Host(args) => run_host(args).await,
        Command::List(args) => list(args).await,
        Command::Delete(args) => delete(args).await,
    }
}

/// `create`: container + injected binary, NOT started; exit. The
/// exists case is a HARD error — creating an id twice is a caller bug.
async fn create(args: CreateArgs) {
    let objectiveai_dir = resolve_objectiveai_dir(&args.objectiveai_dir);
    let bin_dir = objectiveai_dir.join("bin");
    let mounts: Vec<podman::laboratory::Mount> = args
        .mounts
        .iter()
        .filter_map(|m| {
            m.rsplit_once(':').map(|(host, container)| podman::laboratory::Mount {
                host: host.to_string(),
                container: container.to_string(),
            })
        })
        .collect();
    let env: Vec<(String, String)> = args
        .env
        .iter()
        .filter_map(|e| e.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
        .collect();

    let podman = podman::Podman::new(bin_dir.clone());
    let laboratory_binary = bin_dir.join("objectiveai-mcp-laboratory");
    if let Err(e) = podman::laboratory::create(
        &podman,
        &args.objectiveai_state,
        &laboratory_binary,
        &args.id,
        &args.image,
        &mounts,
        &env,
        &args.cwd,
    )
    .await
    {
        let message = e.0.to_ascii_lowercase();
        if message.contains("already in use") || message.contains("already exists") {
            eprintln!("laboratory '{}' already exists", args.id);
        } else {
            eprintln!("create laboratory '{}': {e}", args.id);
        }
        std::process::exit(1);
    }
}

/// `list`: podman label read → JSON array on stdout. Same on-demand
/// podman install/machine semantics the manager itself has.
async fn list(args: ListArgs) {
    let objectiveai_dir = resolve_objectiveai_dir(&args.objectiveai_dir);
    let podman = podman::Podman::new(objectiveai_dir.join("bin"));
    let labs = match podman::laboratory::list(&podman, &args.objectiveai_state).await {
        Ok(labs) => labs,
        Err(e) => {
            eprintln!("list laboratories: {e}");
            std::process::exit(1);
        }
    };
    let identities: Vec<Identify> = labs.into_iter().map(identify_from_info).collect();
    match serde_json::to_string(&identities) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("serialize laboratories: {e}");
            std::process::exit(1);
        }
    }
}

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
    }
}

/// `delete`: force-remove the container (`podman rm -f`) + exit.
async fn delete(args: DeleteArgs) {
    let objectiveai_dir = resolve_objectiveai_dir(&args.objectiveai_dir);
    let podman = podman::Podman::new(objectiveai_dir.join("bin"));
    if let Err(e) =
        podman::laboratory::remove(&podman, &args.objectiveai_state, &args.id).await
    {
        eprintln!("delete laboratory: {e}");
        std::process::exit(1);
    }
}

/// `host`: THE resident laboratory host for this (machine, state).
async fn run_host(args: HostArgs) {
    // Per-address signatures ride ONE env var (never argv): a JSON map
    // address → signature, holding only the addresses that HAVE one.
    let signatures: std::collections::HashMap<String, String> =
        std::env::var("DAEMON_SIGNATURES")
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();

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
