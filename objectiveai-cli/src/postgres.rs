//! Stage 1 embedded-postgres bootstrap.
//!
//! On CLI startup, ensures a postmaster is alive for the current
//! `<config_base_dir>/db/` data dir. Subsequent invocations (or
//! concurrent siblings) see a live socket / port and return
//! immediately.
//!
//! - **Transport — Unix**: `listen_addresses = ''` so postgres opens
//!   zero TCP listeners; the Unix socket lives at
//!   `<data_dir>/.s.PGSQL.1` (the `port = 1` setting is purely the
//!   socket-filename suffix). Nothing binds to TCP.
//! - **Transport — Windows**: TCP loopback at `127.0.0.1:5432`.
//!   `postgresql_embedded::Settings::socket_dir` is documented as
//!   "Unix-only; ignored on Windows", and tokio's `UnixStream` /
//!   `UnixListener` are `#[cfg(unix)]`-gated, so Windows takes the
//!   loopback path. Real TCP port — port 1 isn't viable on Windows
//!   (privileged).
//! - **Lock**: `interprocess::local_socket` — cross-platform
//!   (Unix-domain socket on Unix, named pipe on Windows). Mirrors
//!   the `bind_or_busy` idiom from `crate::instance::pipes`.
//! - **Lifetime**: `std::mem::forget` the handle after `start()` so
//!   the crate's `Drop` impl never calls `pg_ctl stop`. `pg_ctl`
//!   itself daemonizes the postmaster (double-fork on Unix; detached
//!   process on Windows), so the postmaster outlives every CLI
//!   process — it becomes an orphan reparented to PID 1 on Unix and
//!   keeps running on Windows until explicitly stopped.
//!
//! Stage 1 does not connect to postgres, create databases, or run
//! migrations. It only ensures the postmaster is alive.

use std::path::{Path, PathBuf};
use std::time::Duration;

use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions, ToFsName};

use crate::error::Error;

/// Used purely as the socket-filename suffix on Unix
/// (`.s.PGSQL.1`); `listen_addresses = ''` means no TCP listener
/// binds to this number.
#[cfg(unix)]
const PG_PORT: u16 = 1;
/// Real TCP port on `127.0.0.1` on Windows. Port 1 isn't viable
/// (privileged); 5432 is postgres's standard.
#[cfg(windows)]
const PG_PORT: u16 = 5432;

const PROBE_TIMEOUT: Duration = Duration::from_millis(250);
const LOSER_POLL_INTERVAL: Duration = Duration::from_millis(100);
const LOSER_POLL_BUDGET: Duration = Duration::from_secs(60);

/// Probe the postgres transport (Unix socket on Unix, TCP loopback
/// on Windows); if not alive, single-flight-spawn postgres and
/// detach it. Returns `Ok` once a postmaster is reachable.
pub async fn bootstrap(config_base_dir: &Path) -> Result<(), Error> {
    let data_dir = config_base_dir.join("db");
    let install_dir = data_dir.join("installation");
    let lock_path = data_dir.join("spawn.lock.sock");

    if probe_alive(&data_dir).await {
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
                if probe_alive(&data_dir).await {
                    return Ok(());
                }
                spawn_and_forget(&data_dir, &install_dir).await?;
                return Ok(());
            }
            LockOutcome::Loser => return wait_for_alive(&data_dir).await,
            LockOutcome::Stale => {
                // Best-effort cleanup; Unix sockets leave a stale
                // file when the owning process crashes. Named pipes
                // on Windows auto-clean, so the remove is a no-op
                // there but harmless.
                #[cfg(unix)]
                let _ = tokio::fs::remove_file(&lock_path).await;
                continue;
            }
        }
    }
    Err(Error::PostgresBootstrap(
        "could not acquire spawn lock after 4 attempts".to_string(),
    ))
}

#[cfg(unix)]
async fn probe_alive(data_dir: &Path) -> bool {
    let sock = data_dir.join(format!(".s.PGSQL.{PG_PORT}"));
    matches!(
        tokio::time::timeout(PROBE_TIMEOUT, tokio::net::UnixStream::connect(&sock)).await,
        Ok(Ok(_))
    )
}

#[cfg(windows)]
async fn probe_alive(_data_dir: &Path) -> bool {
    matches!(
        tokio::time::timeout(
            PROBE_TIMEOUT,
            tokio::net::TcpStream::connect(("127.0.0.1", PG_PORT)),
        )
        .await,
        Ok(Ok(_))
    )
}

enum LockOutcome {
    /// We hold the lock; the listener is dropped at end of scope
    /// to release it.
    Acquired(interprocess::local_socket::tokio::Listener),
    /// Sibling process is the elected bootstrapper.
    Loser,
    /// The lock endpoint exists but no peer is listening — leftover
    /// from a crashed bootstrapper. Caller should `unlink` + retry.
    Stale,
}

async fn try_bind_lock(lock_path: &Path) -> LockOutcome {
    let owned_path: PathBuf = lock_path.to_path_buf();
    let name = match owned_path.to_fs_name::<GenericFilePath>() {
        Ok(n) => n,
        Err(_) => return LockOutcome::Stale,
    };
    match ListenerOptions::new().name(name).create_tokio() {
        Ok(listener) => LockOutcome::Acquired(listener),
        Err(e) if is_addr_in_use(&e) => {
            // Probe the lock to distinguish "live sibling" from
            // "stale endpoint left by a crashed sibling".
            let probe_name = match lock_path.to_path_buf().to_fs_name::<GenericFilePath>() {
                Ok(n) => n,
                Err(_) => return LockOutcome::Stale,
            };
            let live = tokio::time::timeout(
                PROBE_TIMEOUT,
                interprocess::local_socket::tokio::Stream::connect(probe_name),
            )
            .await
            .is_ok_and(|r| r.is_ok());
            if live {
                LockOutcome::Loser
            } else {
                LockOutcome::Stale
            }
        }
        Err(_) => LockOutcome::Stale,
    }
}

fn is_addr_in_use(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    if matches!(e.kind(), ErrorKind::AddrInUse | ErrorKind::AlreadyExists) {
        return true;
    }
    if let Some(code) = e.raw_os_error() {
        if cfg!(windows) && (code == 231 || code == 5) {
            // ERROR_PIPE_BUSY (231) / ERROR_ACCESS_DENIED (5) — the
            // named-pipe equivalent of `AddrInUse`.
            return true;
        }
    }
    false
}

async fn wait_for_alive(data_dir: &Path) -> Result<(), Error> {
    let deadline = std::time::Instant::now() + LOSER_POLL_BUDGET;
    while std::time::Instant::now() < deadline {
        if probe_alive(data_dir).await {
            return Ok(());
        }
        tokio::time::sleep(LOSER_POLL_INTERVAL).await;
    }
    Err(Error::PostgresBootstrap(
        "timed out waiting for sibling-process postgres to come up".to_string(),
    ))
}

async fn spawn_and_forget(data_dir: &Path, install_dir: &Path) -> Result<(), Error> {
    let mut settings = postgresql_embedded::Settings::default();
    settings.data_dir = PathBuf::from(data_dir);
    settings.installation_dir = PathBuf::from(install_dir);
    settings.port = PG_PORT;
    settings.temporary = false;

    #[cfg(unix)]
    {
        settings.socket_dir = Some(PathBuf::from(data_dir));
        settings
            .configuration
            .insert("listen_addresses".into(), "".into());
        settings.configuration.insert(
            "unix_socket_directories".into(),
            data_dir.display().to_string(),
        );
    }
    #[cfg(windows)]
    {
        settings.host = "127.0.0.1".to_string();
        settings
            .configuration
            .insert("listen_addresses".into(), "127.0.0.1".into());
    }

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
    // `pg_ctl stop` on this process's exit. The postmaster has
    // already been daemonized by `pg_ctl start` (double-fork on
    // Unix → reparented to init; detached child on Windows), so
    // `mem::forget` keeps it alive past CLI exit — every subsequent
    // CLI invocation skips the spawn and reuses the live socket.
    std::mem::forget(pg);
    Ok(())
}
