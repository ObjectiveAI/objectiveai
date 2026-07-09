//! `objectiveai-laboratory` — the laboratory manager binary.
//!
//! Owns ONE laboratory: holds the `<state>/locks/laboratories/<id>`
//! file lock (the id is unique per state, so no full-path key is
//! needed), drives the podman container (create-if-absent → inject the
//! MCP binary → start — the exact logic that used to live CLI-side),
//! and dials OUT to the daemon's `/laboratory` WebSocket where it
//! IDENTIFIES itself first and authorizes second. From then on it is a
//! mini-conduit: the daemon forwards MCP + transfer requests here and
//! this process serves them against its container.
//!
//! Everything is a clap argument except the authorization signature,
//! which arrives via the `DAEMON_SIGNATURE` environment variable (the
//! same consumer-side convention the viewer uses) so it never appears
//! in a process listing.

mod channel;
mod podman;
mod server;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{Identify, IdentifyMount};

#[derive(Parser)]
#[command(name = "objectiveai-laboratory", version)]
struct Args {
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
    /// The daemon's `ws://` base address (its `/laboratory` route is
    /// appended).
    #[arg(long)]
    daemon_address: String,
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

#[tokio::main]
async fn main() {
    // Load a `.env` if present BEFORE reading DAEMON_SIGNATURE.
    let _ = dotenv::dotenv();
    let args = Args::parse();
    let signature = std::env::var("DAEMON_SIGNATURE").ok();

    let objectiveai_dir = args.objectiveai_dir.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".objectiveai")
    });
    let bin_dir = objectiveai_dir.join("bin");
    let state_dir = objectiveai_dir
        .join("state")
        .join(&args.objectiveai_state);

    // ── The laboratory id lock ───────────────────────────────────
    // One manager per (state, id), enforced by the same kernel-backed
    // lockfile the rest of the system uses; released on any death.
    let lock_dir = state_dir.join("locks").join("laboratories");
    let claim = match objectiveai_sdk::lockfile::try_acquire(
        &lock_dir,
        &args.id,
        &format!("manager pid {}", std::process::id()),
    )
    .await
    {
        Some(claim) => claim,
        None => {
            eprintln!(
                "laboratory '{}' is already managed (its lock is held) — exiting",
                args.id
            );
            std::process::exit(1);
        }
    };

    // ── Container up (the former CLI-side logic, relocated) ──────
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
    // Create-if-absent: a manager restarting onto an existing container
    // skips creation (podman's name-in-use error reads as "exists") and
    // just starts it.
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
        if !(message.contains("already in use") || message.contains("already exists")) {
            eprintln!("create laboratory '{}': {e}", args.id);
            let _ = claim.release();
            std::process::exit(1);
        }
    }
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
            let _ = claim.release();
            std::process::exit(1);
        }
    };
    if !args.suppress_output {
        eprintln!("laboratory '{}' on 127.0.0.1:{port}", args.id);
    }

    // ── Serve until killed ───────────────────────────────────────
    let identify = Identify {
        id: args.id.clone(),
        image: args.image.clone(),
        mounts: mounts
            .iter()
            .map(|m| IdentifyMount {
                host: m.host.clone(),
                container: m.container.clone(),
            })
            .collect(),
        env: env.iter().map(|(k, v)| [k.clone(), v.clone()]).collect(),
        cwd: args.cwd.clone(),
    };
    let server = Arc::new(server::LabServer::new(format!("http://127.0.0.1:{port}")));
    channel::run(
        args.daemon_address.clone(),
        identify,
        signature,
        server,
        args.suppress_output,
    )
    .await;
    // channel::run never returns; the claim lives for the process.
    let _ = claim.release();
}
