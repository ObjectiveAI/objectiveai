//! `objectiveai-db` — the ObjectiveAI database server: a thin vehicle
//! around embedded PostgreSQL.
//!
//! Running this binary ensures a postmaster is alive for the
//! `<OBJECTIVEAI_DIR>/state/<OBJECTIVEAI_STATE>/db/` data dir, bound
//! to a FIXED address+port from the environment, then prints
//! `listening on <addr>:<port>` to stderr and exits. The postmaster itself is daemonized by `pg_ctl`
//! (double-fork on Unix → reparented to init; detached child on
//! Windows) and outlives this process — the binary is a launcher, not
//! a supervisor. Re-running against a live postmaster is a fast
//! no-op that re-prints the listening line.
//!
//! Environment (all optional; the spawn helper in objectiveai-cli
//! passes ADDRESS/PORT and forwards the rest from `config db` + its
//! own resolved dir/state):
//!
//!   OBJECTIVEAI_DIR    layout root (default `~/.objectiveai`). The
//!                      postgres binaries extract ONCE per machine to
//!                      `<dir>/bin/pg-bin/`, shared by every state.
//!   OBJECTIVEAI_STATE  state name (default `default`, restricted to
//!                      `[A-Za-z0-9_-]+`). The cluster lives at
//!                      `<dir>/state/<state>/db/`, password file at
//!                      `<dir>/state/<state>/.pgpass`, bootstrap lock
//!                      at `<dir>/state/<state>/db.lock` — one
//!                      database per state.
//!   ADDRESS          bind address (default `127.0.0.1`)
//!   PORT             bind port (default `5433` — one off the system
//!                    postgres default so the two coexist)
//!   PASSWORD         superuser password the cluster is initdb'd
//!                    with (default `objectiveai`). Only applied on
//!                    the FIRST initdb of a data dir — an existing
//!                    cluster keeps the password it was created with.
//!
//! Readiness probing is a plain TCP connect to the configured
//! address:port — anything already listening there is treated as the
//! database being alive (same trade-off the cli's old in-process
//! bootstrap made).
//!
//! Stage 1 only: no databases are created and no schema is applied
//! here — objectiveai-cli does that on connect.

mod lock_file;

use std::path::{Path, PathBuf};
use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_millis(250);

struct Env {
    /// `<OBJECTIVEAI_DIR>/bin` — the machine-wide binaries tree the
    /// postgres install extracts into (`pg-bin/`).
    bin_dir: PathBuf,
    /// `<OBJECTIVEAI_DIR>/state/<OBJECTIVEAI_STATE>` — the per-state
    /// tree holding the cluster, password file, and bootstrap lock.
    state_dir: PathBuf,
    address: String,
    port: u16,
    password: String,
}

fn read_env() -> Result<Env, String> {
    let dir = match std::env::var("OBJECTIVEAI_DIR") {
        Ok(v) if !v.trim().is_empty() => PathBuf::from(v),
        _ => dirs::home_dir()
            .ok_or("OBJECTIVEAI_DIR unset and no home directory found")?
            .join(".objectiveai"),
    };
    let state = match std::env::var("OBJECTIVEAI_STATE") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => "default".to_string(),
    };
    // State names become directory names under <dir>/state/ — reject
    // separators, dot-segments, and anything else outside the safe
    // charset (mirrors objectiveai-cli's validation).
    if state.is_empty()
        || !state
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "OBJECTIVEAI_STATE {state:?} is invalid: state names must match [A-Za-z0-9_-]+"
        ));
    }
    let address = match std::env::var("ADDRESS") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => "127.0.0.1".to_string(),
    };
    let port = match std::env::var("PORT") {
        Ok(v) if !v.trim().is_empty() => v
            .trim()
            .parse::<u16>()
            .map_err(|e| format!("PORT {v:?} is not a valid port: {e}"))?,
        _ => 5433,
    };
    let password = match std::env::var("PASSWORD") {
        Ok(v) if !v.is_empty() => v,
        _ => "objectiveai".to_string(),
    };
    Ok(Env {
        bin_dir: dir.join("bin"),
        state_dir: dir.join("state").join(state),
        address,
        port,
        password,
    })
}

#[tokio::main]
async fn main() {
    let env = match read_env() {
        Ok(env) => env,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = bootstrap(&env).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
    // The readiness line objectiveai-cli's `db spawn` waits for
    // (`spawn_and_wait_for_listening` matches on "listening",
    // case-insensitive) — same protocol as objectiveai-api and
    // objectiveai-viewer.
    eprintln!("listening on {}:{}", env.address, env.port);
}

/// Probe the configured address:port; if nothing is listening,
/// single-flight-spawn postgres and detach it. Returns `Ok` once a
/// postmaster is reachable. The binaries install lands in the shared
/// `<bin_dir>/pg-bin/`; cluster data (`db/`), password file
/// (`.pgpass`), and bootstrap lock (`db.lock`) are per-state under
/// `state_dir`.
///
/// The single-flight loop is the same event-driven claim/wait shape
/// objectiveai-cli used when it bootstrapped postgres in-process:
/// every iteration either finds postgres alive (return), acquires the
/// lock (spawn, return), or blocks kernel-signaled until the current
/// holder releases, then re-evaluates. A crashed bootstrapper's lock
/// auto-releases via OS cleanup, so a waiter takes over.
async fn bootstrap(env: &Env) -> Result<(), String> {
    let data_dir = env.state_dir.join("db");
    let install_dir = env.bin_dir.join("pg-bin");
    // At the root of the state dir, NOT inside `db/` (initdb refuses
    // a non-empty data dir) and NOT inside `pg-bin/` (`pg.setup()`
    // early-returns from `install()` if `installation_dir.exists()`,
    // so an empty `pg-bin/` with just our lock file in it would
    // silently skip the extract).
    let lock_path = env.state_dir.join("db.lock");
    tokio::fs::create_dir_all(&env.state_dir)
        .await
        .map_err(|e| format!("mkdir {:?}: {e}", env.state_dir))?;

    loop {
        if probe_alive(&env.address, env.port).await {
            return Ok(());
        }

        tokio::fs::create_dir_all(&data_dir)
            .await
            .map_err(|e| format!("mkdir {data_dir:?}: {e}"))?;

        match lock_file::try_acquire(&lock_path) {
            Some(_claim) => {
                // Re-probe inside the claim — closes the race where a
                // sibling won, finished, dropped their claim, and we
                // landed in `try_acquire` just after the kernel
                // released their slot.
                if probe_alive(&env.address, env.port).await {
                    return Ok(());
                }
                spawn_and_forget(env, &data_dir, &install_dir).await?;
                // `_claim` drops here → kernel releases the lock →
                // any waiter wakes from `wait_release`.
                return Ok(());
            }
            None => {
                lock_file::wait_release(&lock_path)
                    .await
                    .map_err(|e| format!("wait_release({lock_path:?}): {e}"))?;
                // fall through → next loop iteration
            }
        }
    }
}

/// `true` if a TCP connect to the configured address:port succeeds
/// within [`PROBE_TIMEOUT`]. Wildcard bind addresses are probed via
/// loopback (you can't connect TO `0.0.0.0`).
async fn probe_alive(address: &str, port: u16) -> bool {
    let host = match address {
        "0.0.0.0" | "::" => "127.0.0.1",
        other => other,
    };
    matches!(
        tokio::time::timeout(
            PROBE_TIMEOUT,
            tokio::net::TcpStream::connect((host, port)),
        )
        .await,
        Ok(Ok(_))
    )
}

async fn spawn_and_forget(
    env: &Env,
    data_dir: &Path,
    install_dir: &Path,
) -> Result<(), String> {
    // With the `bundled` feature, the postgres archive ships embedded
    // in this binary (~163M — the whole reason this vehicle exists as
    // a separate binary instead of inside objectiveai-cli);
    // `pg.setup()` extracts from those static bytes into
    // `install_dir` on first run — no network, no shared theseus
    // cache.
    let mut settings = postgresql_embedded::Settings::default();
    settings.installation_dir = PathBuf::from(install_dir);
    settings.data_dir = PathBuf::from(data_dir);
    // `Settings::default()` puts the password file in
    // `tempfile::tempdir()` (OS temp root). Pin it next to `data_dir`
    // (NOT inside — initdb refuses to run against a non-empty data
    // directory) so per-state writes stay inside the state dir.
    settings.password_file = data_dir
        .parent()
        .map(|p| p.join(".pgpass"))
        .unwrap_or_else(|| PathBuf::from(".pgpass"));
    // Fixed bind — the port comes from config (via the cli's
    // `db spawn`) instead of the old OS-assigned-ephemeral approach,
    // so objectiveai-cli can build its connection URL straight from
    // `config db` without reading `postmaster.pid`.
    settings.port = env.port;
    settings.host = env.address.clone();
    settings.temporary = false;
    settings.password = env.password.clone();
    // The crate's default `timeout` (5s) covers the slowest of
    // `install`, `initialize`, and `start`. `initialize` (initdb)
    // routinely takes 10–30s on first run; install can take 30–60s
    // while the archive is extracted. Give all three a generous
    // budget.
    settings.timeout = Some(Duration::from_secs(180));
    settings
        .configuration
        .insert("listen_addresses".into(), env.address.clone());
    settings
        .configuration
        .insert("logging_collector".into(), "on".into());
    settings
        .configuration
        .insert("log_directory".into(), "log".into());

    let mut pg = postgresql_embedded::PostgreSQL::new(settings);
    pg.setup().await.map_err(|e| format!("setup: {e}"))?;
    pg.start().await.map_err(|e| format!("start: {e}"))?;
    // Defeat the crate's `Drop` impl: it would otherwise call
    // `pg_ctl stop` on this process's exit. `pg_ctl start` has
    // already daemonized the postmaster, so `mem::forget` keeps it
    // alive past this launcher's exit.
    std::mem::forget(pg);
    Ok(())
}
