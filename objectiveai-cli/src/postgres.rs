//! Stage 1 embedded-postgres bootstrap.
//!
//! On CLI startup, ensures a postmaster is running at
//! `<config_base_dir>/db/.s.PGSQL.1` and detaches it so it outlives
//! every CLI process. Subsequent invocations (or concurrent siblings)
//! see a live socket and return immediately.
//!
//! - **Transport**: Unix socket only. `listen_addresses = ''` so
//!   postgres opens zero TCP listeners. The `port = 1` setting is
//!   purely the socket-filename suffix (`.s.PGSQL.1`); nothing binds
//!   to TCP port 1.
//! - **Data dir**: `<config_base_dir>/db/`. Postgres puts its data,
//!   logs, socket, and the downloaded binary install all under here.
//! - **Single-flight**: bind a sentinel Unix socket at
//!   `<data_dir>/spawn.lock.sock` to elect exactly one bootstrapper
//!   per data dir. Mirrors the `bind_or_busy` idiom from
//!   `crate::instance::pipes`.
//! - **Lifetime**: `std::mem::forget` the handle after `start()` so
//!   the crate's `Drop` impl never calls `pg_ctl stop`. The
//!   postmaster outlives every CLI process; subsequent invocations
//!   reuse it.
//!
//! Stage 1 does not connect to postgres, create databases, or run
//! migrations. It only ensures the postmaster is alive.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::Error;

/// Used purely as the suffix of the Unix-socket filename
/// (`.s.PGSQL.1`). `listen_addresses = ''` so no TCP listener binds
/// to this number.
const PG_PORT: u16 = 1;
const PROBE_TIMEOUT: Duration = Duration::from_millis(250);
const LOSER_POLL_INTERVAL: Duration = Duration::from_millis(100);
const LOSER_POLL_BUDGET: Duration = Duration::from_secs(60);

/// Probe the Unix socket; if not alive, single-flight-spawn postgres
/// and detach it. Returns `Ok` once a postmaster is reachable.
#[cfg(unix)]
pub async fn bootstrap(config_base_dir: &Path) -> Result<(), Error> {
    let data_dir = config_base_dir.join("db");
    let install_dir = data_dir.join("installation");
    let sock_path = data_dir.join(format!(".s.PGSQL.{PG_PORT}"));
    let lock_path = data_dir.join("spawn.lock.sock");

    if probe_alive(&sock_path).await {
        return Ok(());
    }

    tokio::fs::create_dir_all(&data_dir)
        .await
        .map_err(|e| Error::PostgresBootstrap(format!("mkdir {data_dir:?}: {e}")))?;

    for _ in 0..4 {
        match try_bind_lock(&lock_path).await {
            LockOutcome::Acquired(_listener) => {
                // Re-probe inside the lock to close the race window
                // where a sibling won, finished, and dropped its
                // listener between our outer probe and the bind.
                if probe_alive(&sock_path).await {
                    return Ok(());
                }
                spawn_and_forget(&data_dir, &install_dir).await?;
                return Ok(());
            }
            LockOutcome::Loser => return wait_for_alive(&sock_path).await,
            LockOutcome::Stale => {
                let _ = tokio::fs::remove_file(&lock_path).await;
                continue;
            }
        }
    }
    Err(Error::PostgresBootstrap(
        "could not acquire spawn lock after 4 attempts".to_string(),
    ))
}

#[cfg(not(unix))]
pub async fn bootstrap(_config_base_dir: &Path) -> Result<(), Error> {
    Err(Error::PostgresBootstrap(
        "embedded postgres is unix-only for stage 1; windows TCP-loopback \
         transport is a follow-up"
            .to_string(),
    ))
}

#[cfg(unix)]
async fn probe_alive(sock_path: &Path) -> bool {
    matches!(
        tokio::time::timeout(PROBE_TIMEOUT, tokio::net::UnixStream::connect(sock_path)).await,
        Ok(Ok(_))
    )
}

#[cfg(unix)]
enum LockOutcome {
    /// We hold the lock; the listener is dropped at end of scope to
    /// release it.
    Acquired(tokio::net::UnixListener),
    /// Sibling process is the elected bootstrapper.
    Loser,
    /// The socket file exists but no peer is listening — leftover
    /// from a crashed bootstrapper. Caller should `unlink` + retry.
    Stale,
}

#[cfg(unix)]
async fn try_bind_lock(lock_path: &Path) -> LockOutcome {
    match tokio::net::UnixListener::bind(lock_path) {
        Ok(listener) => LockOutcome::Acquired(listener),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            if probe_alive(lock_path).await {
                LockOutcome::Loser
            } else {
                LockOutcome::Stale
            }
        }
        Err(_) => LockOutcome::Stale,
    }
}

#[cfg(unix)]
async fn wait_for_alive(sock_path: &Path) -> Result<(), Error> {
    let deadline = std::time::Instant::now() + LOSER_POLL_BUDGET;
    while std::time::Instant::now() < deadline {
        if probe_alive(sock_path).await {
            return Ok(());
        }
        tokio::time::sleep(LOSER_POLL_INTERVAL).await;
    }
    Err(Error::PostgresBootstrap(
        "timed out waiting for sibling-process postgres to come up".to_string(),
    ))
}

#[cfg(unix)]
async fn spawn_and_forget(data_dir: &Path, install_dir: &Path) -> Result<(), Error> {
    let mut settings = postgresql_embedded::Settings::default();
    settings.data_dir = PathBuf::from(data_dir);
    settings.installation_dir = PathBuf::from(install_dir);
    settings.port = PG_PORT;
    settings.temporary = false;
    settings
        .configuration
        .insert("listen_addresses".into(), "".into());
    settings.configuration.insert(
        "unix_socket_directories".into(),
        data_dir.display().to_string(),
    );
    settings
        .configuration
        .insert("logging_collector".into(), "on".into());
    settings
        .configuration
        .insert("log_directory".into(), "log".into());

    let mut pg = postgresql_embedded::PostgreSQL::new(settings);
    pg.setup()
        .await
        .map_err(|e| Error::PostgresBootstrap(format!("setup: {e}")))?;
    pg.start()
        .await
        .map_err(|e| Error::PostgresBootstrap(format!("start: {e}")))?;
    // Defeat the crate's `Drop` impl: it would otherwise call
    // `pg_ctl stop` on this process's exit. We want the postmaster
    // to survive — subsequent CLI invocations skip the spawn and
    // just reuse the live socket.
    std::mem::forget(pg);
    Ok(())
}
