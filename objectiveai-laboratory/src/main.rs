//! `objectiveai-laboratory` — the laboratory manager binary.
//!
//! Three subcommands:
//!
//! - **`create`** — create the laboratory container (podman create +
//!   inject the MCP binary; NOT started) and exit. Errors if the id
//!   already exists in this state. Pure and daemonless: no lock, no
//!   WebSocket, no signature.
//! - **`connect`** — the resident manager for one `(id, address)`
//!   pair: rebuild the lab's identity from its container label, hold
//!   the `<state>/locks/laboratories/<id>.<base62(xxh3(address))>`
//!   lock (one manager per laboratory per daemon; simultaneous
//!   connects to the same pair resolve to exactly one winner), start
//!   the container, and dial `<address>/laboratory` — IDENTIFY first,
//!   authorize second — serving MCP + transfer requests until killed.
//!   On graceful shutdown the container is STOPPED (never removed).
//!   The same laboratory may be connected to several daemons at once,
//!   one manager process per address.
//! - **`list`** — print the state's laboratory containers (running or
//!   not) as a JSON array of identity objects on stdout.
//!
//! Everything is a clap argument except the authorization signature,
//! which arrives via the `DAEMON_SIGNATURE` environment variable (the
//! same consumer-side convention the viewer uses) so it never appears
//! in a process listing.

mod channel;
mod cleaner;
mod podman;
mod server;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{
    connect_lock_key, Identify, IdentifyMount,
};

#[derive(Parser)]
#[command(name = "objectiveai-laboratory", version)]
enum Command {
    /// Create the laboratory container (not started); errors if the id
    /// already exists in this state.
    Create(CreateArgs),
    /// Run the resident manager: connect a CREATED laboratory to a
    /// daemon (until killed).
    Connect(ConnectArgs),
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
struct ConnectArgs {
    /// The laboratory id (must already be created).
    #[arg(long)]
    id: String,
    /// The daemon's `ws://` base address to connect to (its
    /// `/laboratory` route is appended).
    #[arg(long)]
    address: String,
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
    // Load a `.env` if present BEFORE reading DAEMON_SIGNATURE.
    let _ = dotenv::dotenv();
    match Command::parse() {
        Command::Create(args) => create(args).await,
        Command::Connect(args) => connect(args).await,
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

fn identify_from_info(lab: podman::laboratory::LaboratoryInfo) -> Identify {
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

/// `connect`: the resident manager for one `(id, address)` pair.
async fn connect(args: ConnectArgs) {
    let signature = std::env::var("DAEMON_SIGNATURE").ok();

    let objectiveai_dir = resolve_objectiveai_dir(&args.objectiveai_dir);
    let bin_dir = objectiveai_dir.join("bin");
    let state_dir = objectiveai_dir
        .join("state")
        .join(&args.objectiveai_state);
    let podman = podman::Podman::new(bin_dir.clone());

    // ── Identity from the container label (create is a prerequisite) ─
    let labs = match podman::laboratory::list(&podman, &args.objectiveai_state).await {
        Ok(labs) => labs,
        Err(e) => {
            eprintln!("read laboratories: {e}");
            std::process::exit(1);
        }
    };
    let Some(info) = labs.into_iter().find(|l| l.id == args.id) else {
        eprintln!(
            "laboratory '{}' is not created — run `laboratories create` first",
            args.id
        );
        std::process::exit(1);
    };
    let identify = identify_from_info(info);

    // ── The (id, address) connection lock, under the id GUARD ────
    // Guard first (BLOCKING, bare id — no url hash): the cleaner holds
    // this same guard around its check-locks-then-stop window, so a
    // connect can never slip its lock acquisition between a cleaner's
    // "no locks held" observation and its `podman stop`. Released
    // explicitly the moment the connection lock is held — a manager
    // past the guard is always visible to the cleaner's veto check.
    let lock_dir = state_dir.join("locks").join("laboratories");
    let guard = match objectiveai_sdk::lockfile::wait_acquire(
        &lock_dir,
        &args.id,
        &format!("guard pid {}", std::process::id()),
    )
    .await
    {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("laboratory '{}' guard lock: {e}", args.id);
            std::process::exit(1);
        }
    };
    // One manager per laboratory per daemon address; simultaneous
    // connects to the same pair race this try_acquire and exactly one
    // wins (the loser's spawner re-probes the published lock — the
    // api/db/mcp spawn discipline).
    let lock_key = connect_lock_key(&args.id, &args.address);
    let claim = match objectiveai_sdk::lockfile::try_acquire(
        &lock_dir,
        &lock_key,
        &format!("manager pid {}", std::process::id()),
    )
    .await
    {
        Some(claim) => {
            if let Err(e) = guard.release() {
                eprintln!("laboratory '{}' guard release: {e}", args.id);
                let _ = claim.release();
                std::process::exit(1);
            }
            claim
        }
        None => {
            let _ = guard.release();
            eprintln!(
                "laboratory '{}' is already connected to {} (its lock is held) — exiting",
                args.id, args.address
            );
            std::process::exit(1);
        }
    };

    // ── Container up (start-not-create; a stopped container resumes) ─
    if let Err(e) =
        podman::laboratory::start(&podman, &args.objectiveai_state, &args.id).await
    {
        eprintln!("start laboratory '{}': {e}", args.id);
        let _ = claim.release();
        std::process::exit(1);
    }
    let port = match podman::laboratory::host_port(
        &podman,
        &args.objectiveai_state,
        &args.id,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("laboratory '{}' port: {e}", args.id);
            // We just started it — don't leak a running container.
            let _ =
                podman::laboratory::stop(&podman, &args.objectiveai_state, &args.id).await;
            let _ = claim.release();
            std::process::exit(1);
        }
    };
    if !args.suppress_output {
        eprintln!("laboratory '{}' on 127.0.0.1:{port} → {}", args.id, args.address);
    }

    // ── Serve until killed ───────────────────────────────────────
    let server = Arc::new(server::LabServer::new(format!("http://127.0.0.1:{port}")));

    // Serve until a graceful-shutdown signal, then STOP (never remove)
    // the container: it and its filesystem survive for the next
    // manager to `start` again. A hard kill skips this — the container
    // then keeps running until someone stops it.
    let cleaner_bin_dir = bin_dir.clone();
    let cleaner_state = args.objectiveai_state.clone();
    let cleaner_lock_dir = lock_dir.clone();
    let on_first_connect: Box<dyn FnOnce() + Send> = Box::new(move || {
        tokio::spawn(cleaner::sweep(
            cleaner_bin_dir,
            cleaner_state,
            cleaner_lock_dir,
        ));
    });
    tokio::select! {
        _ = channel::run(
            args.address.clone(),
            identify,
            signature,
            server,
            args.suppress_output,
            on_first_connect,
        ) => {}
        _ = shutdown_signal() => {
            if !args.suppress_output {
                eprintln!("shutting down: stopping laboratory '{}'", args.id);
            }
        }
    }
    if let Err(e) =
        podman::laboratory::stop(&podman, &args.objectiveai_state, &args.id).await
    {
        eprintln!("stop laboratory '{}': {e}", args.id);
    }
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
