//! Stage 1 embedded-postgres bootstrap.
//!
//! On CLI startup, ensures a postmaster is alive for the current
//! `<config_base_dir>/db/` data dir. Subsequent invocations (or
//! concurrent siblings) see a live postmaster and return
//! immediately.
//!
//! - **Transport**: TCP loopback only, on every platform. Port
//!   is OS-assigned via `Settings::port = 0` — the
//!   `postgresql_embedded` crate's `start()` impl binds an
//!   ephemeral free port via `TcpListener::bind(("0.0.0.0", 0))`
//!   internally and hands the resolved port to `pg_ctl`.
//!   Postgres writes the chosen port to
//!   `<data_dir>/postmaster.pid` line 4 on every successful
//!   start (format documented and stable across platforms);
//!   every subsequent CLI invocation reads from there to
//!   discover the live port.
//!
//!   Unix domain sockets used to be the Unix transport here.
//!   They were dropped to collapse the platform split: one
//!   transport everywhere, no `#[cfg]`-gated probe paths, no
//!   `socket_dir` setting that `postgresql_embedded` silently
//!   ignores on Windows, no `tokio::net::UnixStream`
//!   `#[cfg(unix)]` carve-out on the client side.
//!
//! - **Filesystem sandboxing**: every byte the cli writes lives
//!   under `<config_base_dir>/`. The postgres binaries extract
//!   to `<config_base_dir>/db-bin/` (NOT the crate's default
//!   `~/.theseus/postgresql/`); the cluster data lives in
//!   `<config_base_dir>/db/`; the password file at
//!   `<config_base_dir>/.pgpass`; the bootstrap lock at
//!   `<config_base_dir>/db.lock`. Two cli invocations against
//!   distinct `config_base_dir`s share no on-disk state and
//!   never contend for the bootstrap lock. The `bundled`
//!   archive ships in our binary, so the extract is purely
//!   local — no network, no shared cache.
//!
//! - **Lock**: [`crate::lock_file`] — the same OS-level claim-
//!   file primitive `agents::message` / `agent_registry` use
//!   (`CreateFileW + FILE_FLAG_DELETE_ON_CLOSE` on Windows,
//!   `O_CREAT|O_EXCL + flock(LOCK_EX)` on Unix). File presence
//!   ⇔ live owner; OS handles cleanup on process death. Used
//!   only as a bootstrap mutex to serialize concurrent first
//!   spawns against the same `data_dir`; after a postmaster is
//!   alive every subsequent cli skips the lock entirely.
//!
//! - **Lifetime**: `std::mem::forget` the handle after `start()`
//!   so the crate's `Drop` impl never calls `pg_ctl stop`.
//!   `pg_ctl` itself daemonizes the postmaster (double-fork on
//!   Unix → reparented to init; detached process on Windows),
//!   so the postmaster outlives every CLI process — every
//!   subsequent invocation reads `postmaster.pid`, finds the
//!   listening port, and reconnects.
//!
//! Stage 1 does not connect to postgres, create databases, or
//! run migrations. It only ensures the postmaster is alive.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::Error;

const PROBE_TIMEOUT: Duration = Duration::from_millis(250);

/// Read line 4 of `<data_dir>/postmaster.pid`. Postgres writes
/// this file on every successful start; format is documented
/// and stable across platforms:
///
/// ```text
///   Line 1: PID
///   Line 2: Data directory absolute path
///   Line 3: Postmaster start time (epoch)
///   Line 4: Port number
///   Line 5: Socket directory (empty if none)
///   Line 6: First listen_address (empty if listen_addresses='')
///   Line 7: Shared memory key (or 0)
///   Line 8: Postmaster status flag
/// ```
///
/// Returns `None` when the file is missing (postmaster never
/// started, or shut down cleanly), unreadable, or the port line
/// isn't parseable.
async fn read_pid_file_port(data_dir: &Path) -> Option<u16> {
    let pid_file = data_dir.join("postmaster.pid");
    let content = tokio::fs::read_to_string(&pid_file).await.ok()?;
    content.lines().nth(3)?.trim().parse::<u16>().ok()
}

/// Look up the port the postmaster for `data_dir` is listening
/// on. `None` ⇒ no postmaster.pid (or the file's malformed).
/// Used by [`crate::db::init`] to build the connection URL —
/// the bootstrap ran first inside `Context::new`, so by the
/// time `init` reads this the file is in place.
pub(crate) async fn pg_port(data_dir: &Path) -> Option<u16> {
    read_pid_file_port(data_dir).await
}

/// Probe the postgres transport; if not alive, single-flight-
/// spawn postgres and detach it. Returns `Ok` once a postmaster
/// is reachable. Everything stays inside `config_base_dir`:
/// install at `db-bin/`, cluster data at `db/`, password file
/// at `.pgpass`, bootstrap lock at `db.lock`.
pub async fn bootstrap(config_base_dir: &Path) -> Result<(), Error> {
    let data_dir = config_base_dir.join("db");
    let install_dir = config_base_dir.join("db-bin");
    // At the root of `config_base_dir`, NOT inside `db/`
    // (initdb refuses a non-empty data dir) and NOT inside
    // `db-bin/` (the crate's `pg.setup()` early-returns from
    // `install()` if `installation_dir.exists()`, so an empty
    // `db-bin/` with just our lock file in it would silently
    // skip the extract).
    let lock_path = config_base_dir.join("db.lock");
    tokio::fs::create_dir_all(config_base_dir).await.map_err(|e| {
        Error::PostgresBootstrap(format!("mkdir {config_base_dir:?}: {e}"))
    })?;

    // Event-driven retry loop. Every iteration either:
    //   - finds postgres alive → returns Ok
    //   - acquires the lock → spawns postgres → returns Ok
    //   - waits (kernel-signaled via [`crate::lock_file::wait_release`],
    //     not polling) for the current holder to release →
    //     loops, re-evaluates
    //
    // Termination: each `wait_release` consumes one wall-clock
    // interval of "another process holds the lock". After at
    // most ~K parallel sibling cli processes have each had
    // their turn — K = 1–2 in production, ~16 in tests — the
    // loop either finds postgres alive (success) or one of
    // them succeeds in `spawn_and_forget`. A crashed
    // bootstrapper releases its lock via OS cleanup
    // automatically, so the next iteration's `try_acquire`
    // succeeds and we take over.
    loop {
        if probe_alive(&data_dir).await {
            return Ok(());
        }

        tokio::fs::create_dir_all(&data_dir).await.map_err(|e| {
            Error::PostgresBootstrap(format!("mkdir {data_dir:?}: {e}"))
        })?;

        match crate::lock_file::try_acquire(&lock_path) {
            Some(_claim) => {
                // Re-probe inside the claim — closes the race
                // where a sibling won, finished, dropped their
                // claim, and we landed in `try_acquire` just
                // after the kernel released their slot.
                if probe_alive(&data_dir).await {
                    return Ok(());
                }
                spawn_and_forget(&data_dir, &install_dir).await?;
                // `_claim` drops here → kernel releases the
                // lock → any waiter wakes from `wait_release`.
                return Ok(());
            }
            None => {
                // Someone else holds the bootstrap lock right
                // now. Block (kernel-signaled, no polling)
                // until they release it; then loop and
                // re-evaluate. After the release, one of:
                //
                //   1. Holder succeeded → `probe_alive` returns
                //      true → return Ok.
                //   2. Holder failed → `probe_alive` is false →
                //      `try_acquire` succeeds → we take over.
                //   3. Holder released but a third process
                //      grabbed the lock immediately →
                //      `try_acquire` returns `None` →
                //      `wait_release` on the new holder.
                //      Same shape, no special case.
                crate::lock_file::wait_release(&lock_path).await.map_err(|e| {
                    Error::PostgresBootstrap(format!(
                        "wait_release({lock_path:?}): {e}"
                    ))
                })?;
                // fall through → next loop iteration
            }
        }
    }
}

/// `true` if `postmaster.pid` carries a port and a TCP probe to
/// `127.0.0.1:<port>` succeeds within `PROBE_TIMEOUT`. Same
/// code path on every platform.
async fn probe_alive(data_dir: &Path) -> bool {
    let Some(port) = read_pid_file_port(data_dir).await else {
        return false;
    };
    matches!(
        tokio::time::timeout(
            PROBE_TIMEOUT,
            tokio::net::TcpStream::connect(("127.0.0.1", port)),
        )
        .await,
        Ok(Ok(_))
    )
}

/// Hard-coded password the cluster is initdb'd with and that every
/// CLI invocation uses to authenticate. The embedded postgres
/// listens on TCP loopback only, so this isn't security — just
/// deterministic auth that sidesteps the `postgresql_embedded`
/// crate's per-`Settings::default()` random password mint (which
/// we'd have to parse from a file to recover on subsequent
/// starts). With a fixed value, `db::init` can build the
/// connection URL with the password baked in and the cluster's
/// default `pg_hba.conf` works untouched.
pub(crate) const PG_INITDB_PASSWORD: &str = "objectiveai";

async fn spawn_and_forget(data_dir: &Path, install_dir: &Path) -> Result<(), Error> {
    // Pin `installation_dir` either to the env-var override
    // (tests share one install via warmup) or inside
    // `config_base_dir` (production: each base dir gets its own
    // copy, no shared on-disk state outside config_base_dir).
    // With the `bundled` feature, the postgres archive ships
    // embedded in our binary (~163M, accounts for most of the
    // cli's release-build size); `pg.setup()` extracts from
    // those static bytes into `install_dir` on first run — no
    // network, no shared theseus cache.
    let mut settings = postgresql_embedded::Settings::default();
    settings.installation_dir = PathBuf::from(install_dir);
    settings.data_dir = PathBuf::from(data_dir);
    // `Settings::default()` puts the password file in
    // `tempfile::tempdir()` (OS temp root). Pin it next to
    // `data_dir` (NOT inside — initdb refuses to run against a
    // non-empty data directory) so the cli still writes nothing
    // outside `config_base_dir`.
    settings.password_file = data_dir
        .parent()
        .map(|p| p.join(".pgpass"))
        .unwrap_or_else(|| PathBuf::from(".pgpass"));
    // `port = 0` → the crate's `start()` impl binds an ephemeral
    // OS-assigned port via TcpListener and hands it to pg_ctl.
    // The chosen port lands in `postmaster.pid` line 4 — every
    // subsequent CLI invocation reads from there.
    settings.port = 0;
    settings.host = "127.0.0.1".to_string();
    settings.temporary = false;
    settings.password = PG_INITDB_PASSWORD.to_string();
    // The crate's default `timeout` (5s) covers the slowest of
    // `install`, `initialize`, and `start`. `initialize` (which
    // shells out to `initdb`) routinely takes 10–30s on first
    // run; install can take 30–60s while the archive is
    // extracted. Give all three a generous budget.
    settings.timeout = Some(Duration::from_secs(180));
    settings
        .configuration
        .insert("listen_addresses".into(), "127.0.0.1".into());
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
    // `pg_ctl stop` on this process's exit. `pg_ctl start` has
    // already daemonized the postmaster (double-fork on Unix →
    // reparented to init; detached child on Windows), so
    // `mem::forget` keeps it alive past CLI exit — every
    // subsequent CLI invocation reads its port from
    // `postmaster.pid` and reconnects.
    std::mem::forget(pg);
    Ok(())
}
