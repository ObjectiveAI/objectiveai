//! `objectiveai-db` — the ObjectiveAI database server: a resident
//! supervisor around embedded PostgreSQL.
//!
//! Unlike the old launch-and-exit vehicle, this process STAYS ALIVE
//! as the postmaster's parent: postgres runs as a direct child that
//! lives and dies with this process (Windows: a kill-on-close job
//! object; Linux: `PR_SET_PDEATHSIG`; everywhere: `kill_on_drop` for
//! orderly exits). Crash the supervisor and the postmaster goes
//! with it — which couples postmaster liveness to the state lock
//! below, since both end at the same process death.
//!
//! Startup sequence:
//!
//! 1. **Install** (machine-wide, once): if `<dir>/bin/pg-bin` lacks
//!    its completion marker, take a blocking lock on
//!    `<dir>/bin/locks` key `db`, re-check, extract the bundled
//!    archive if still needed, then EXPLICITLY release the lock.
//! 2. **initdb** (per state, once): create the cluster at
//!    `<dir>/state/<state>/db` if it doesn't exist.
//! 3. **Spawn postgres** as a direct child on 127.0.0.1 and a
//!    RANDOM free port — localhost exclusively; nothing is
//!    configurable about the bind.
//! 4. **Fast-acquire the state lock**: `<dir>/state/<state>/locks`
//!    key `db`, publishing the full connection string (including
//!    the random port). Failure means another objectiveai-db
//!    already serves this state → kill the child and exit nonzero.
//! 5. Print `listening on <addr>:<port>` to stderr and wait on the
//!    child forever. Lock release and postmaster death both happen
//!    at process exit, however it happens.
//!
//! Configuration is clap arguments EXCLUSIVELY — no environment
//! variables. All three are required, and they are the only
//! arguments:
//!
//!   --objectiveai-dir <PATH>    layout root. The postgres binaries
//!                               extract ONCE per machine to
//!                               `<dir>/bin/pg-bin/`, shared by
//!                               every state.
//!   --objectiveai-state <NAME>  state name. The cluster lives at
//!                               `<dir>/state/<state>/db/`, password
//!                               file at
//!                               `<dir>/state/<state>/.pgpass` —
//!                               one database per state.
//!   --pg-password <PW>          superuser password the cluster is
//!                               initdb'd with. Only applied on the
//!                               FIRST initdb of a data dir — an
//!                               existing cluster keeps the password
//!                               it was created with.

use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// ObjectiveAI database server — a resident supervisor around
/// embedded PostgreSQL. Binds 127.0.0.1 on a random free port and
/// publishes the connection string in the state's db lock.
#[derive(Parser)]
#[command(name = "objectiveai-db", version)]
struct Args {
    /// Layout root; the postgres install is shared at <dir>/bin/pg-bin.
    #[arg(long)]
    objectiveai_dir: PathBuf,
    /// State name; the cluster lives at <dir>/state/<state>/db.
    #[arg(long)]
    objectiveai_state: String,
    /// Superuser password (applied on the first initdb only).
    #[arg(long)]
    pg_password: String,
}

impl Args {
    fn bin_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("bin")
    }

    fn state_dir(&self) -> PathBuf {
        self.objectiveai_dir
            .join("state")
            .join(&self.objectiveai_state)
    }
}

#[tokio::main]
async fn main() {
    // If launched as a subprocess-reaper guardian (macOS only), run the
    // watch loop and exit before touching args. No-op otherwise.
    objectiveai_sdk::subprocess_reaper::run_guardian_if_invoked();

    let env = Args::parse();
    // `run` either serves forever or fails: every early exit —
    // lock already held, install/initdb/postgres failure, postmaster
    // death — is exit code 1. There is no clean-exit path.
    let e = match run(&env).await {
        Ok(never) => match never {},
        Err(e) => e,
    };
    eprintln!("error: {e}");
    std::process::exit(1);
}

async fn run(env: &Args) -> Result<std::convert::Infallible, String> {
    let lock_dir = env.state_dir().join("locks");

    // 1. Fail fast if this state's db lock is already held: exactly
    //    like the api/mcp/viewer singletons, a supervisor that cannot
    //    own the lock must never report success. (The loser's failure
    //    exit is what the spawning cli's re-probe converts into
    //    success on its side — by the time anyone observes a held
    //    lock, its contents are published.)
    fail_if_db_lock_held(&lock_dir).await?;

    ensure_installed(env).await?;

    // 2. Blocking claim of the init gate. The state's db lock is only
    //    claimed once postgres is READY, so without this gate two
    //    concurrent spawns (two agents in one state) both pass the
    //    spawner's try_read and race initdb / postgres against the
    //    same data dir — the loser dies through the FILESYSTEM before
    //    the winner publishes, which the spawner's re-probe can't
    //    see. The gate serializes the whole init-to-claim window
    //    (same pattern as `ensure_installed`'s shared-extract lock):
    //    fail slow here, fail fast on the final lock.
    let init_claim =
        objectiveai_sdk::lockfile::wait_acquire(&lock_dir, "db-init", "initializing")
            .await
            .map_err(|e| format!("wait_acquire(db-init lock): {e}"))?;

    // 3. Recheck the final lock under the gate — a sibling may have
    //    won while we waited.
    if let Err(e) = fail_if_db_lock_held(&lock_dir).await {
        init_claim
            .release()
            .map_err(|e| format!("release(db-init lock): {e}"))?;
        return Err(e);
    }

    // 4. Init the database and claim+publish the final lock.
    let started = init_and_claim(env, &lock_dir).await;
    // 5. Drop the init gate explicitly — the init-to-claim window is
    //    over whether or not it succeeded.
    init_claim
        .release()
        .map_err(|e| format!("release(db-init lock): {e}"))?;
    let (mut child, port) = started?;

    // The readiness line objectiveai-cli's `db spawn` waits for
    // (`spawn_and_wait_for_listening` matches on "listening",
    // case-insensitive) — same protocol as objectiveai-api and
    // objectiveai-viewer.
    eprintln!("listening on 127.0.0.1:{port}");

    // Resident from here on: live exactly as long as the postmaster.
    // The only success is to keep serving — a postmaster that exits,
    // however cleanly, means this supervisor failed.
    let status = child
        .wait()
        .await
        .map_err(|e| format!("wait on postgres: {e}"))?;
    Err(format!("postgres exited: {status}"))
}

/// Fail with the canonical already-held error when a live sibling
/// owns this state's db lock.
async fn fail_if_db_lock_held(lock_dir: &Path) -> Result<(), String> {
    if objectiveai_sdk::lockfile::try_read(lock_dir, "db")
        .await
        .map_err(|e| format!("try_read(db lock): {e}"))?
        .is_some()
    {
        return Err(
            "another objectiveai-db instance already holds the db lock for this state"
                .to_string(),
        );
    }
    Ok(())
}

/// initdb (if needed) → spawn postgres → wait ready → claim the
/// state's db lock with the connection string. Runs entirely inside
/// the caller's `db-init` sibling lock.
async fn init_and_claim(
    env: &Args,
    lock_dir: &Path,
) -> Result<(tokio::process::Child, u16), String> {
    ensure_initdb(env).await?;

    let port = free_port()?;
    let mut child = spawn_postgres(env, port)?;

    if let Err(e) = wait_ready(port, &mut child).await {
        let _ = child.kill().await;
        return Err(e);
    }

    // Fast-acquire the state lock, publishing the connection string
    // clients use (random port included). The claim is held until
    // THIS process dies — and the postmaster dies with this process
    // too, so lock liveness ⇔ postmaster liveness. Failure means a
    // sibling objectiveai-db already serves this state.
    let contents = connection_string(env, port);
    if objectiveai_sdk::lockfile::try_acquire(lock_dir, "db", &contents)
        .await
        .is_none()
    {
        let _ = child.kill().await;
        return Err(
            "another objectiveai-db instance already holds the db lock for this state"
                .to_string(),
        );
    }
    Ok((child, port))
}

/// Phase 1: ensure the shared postgres install at
/// `<dir>/bin/pg-bin/` exists and is COMPLETE — double-checked
/// behind a blocking lock on `<dir>/bin/locks` key `db` (the same
/// locks dir the api claims its singleton in), explicitly released
/// once the install is in place.
///
/// Completeness is tracked by a `.objectiveai-complete` marker file
/// written after a successful extract: `pg.setup()`'s `install()`
/// early-returns whenever `installation_dir.exists()`, so a partial
/// extract (crash/AV interruption mid-write) would otherwise be
/// silently accepted forever. Missing marker + existing dir ⇒ wipe
/// and re-extract.
async fn ensure_installed(env: &Args) -> Result<(), String> {
    let install_dir = env.bin_dir().join("pg-bin");
    let marker = install_dir.join(".objectiveai-complete");
    // 1. Installed? Done.
    if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
        return Ok(());
    }
    // 2. Take the install lock (blocking — a sibling may be mid-
    //    extract; we wait for it rather than racing it).
    let locks_dir = env.bin_dir().join("locks");
    let claim = objectiveai_sdk::lockfile::wait_acquire(&locks_dir, "db", "installing")
        .await
        .map_err(|e| format!("wait_acquire(db install lock): {e}"))?;
    // 3. Re-check under the lock — the sibling may have finished
    //    while we waited.
    if !tokio::fs::try_exists(&marker).await.unwrap_or(false) {
        // 4. Install. A partial extract has no marker — get it out of
        //    the way first so `install()`'s exists() early-return
        //    can't accept it. Rename-then-delete, NOT delete-in-place:
        //    Windows directory deletion is asynchronous, so a plain
        //    `remove_dir_all` returns while the tree is still
        //    pending-delete and the immediate re-create inside
        //    `pg.setup()` fails with ACCESS_DENIED. The rename frees
        //    the name instantly; deleting the renamed tree is
        //    best-effort (a leftover trash dir is inert and inside
        //    the gitignored pg-bin namespace).
        if tokio::fs::try_exists(&install_dir).await.unwrap_or(false) {
            let trash = env.bin_dir().join(format!(
                "pg-bin.trash-{}",
                std::process::id()
            ));
            tokio::fs::rename(&install_dir, &trash)
                .await
                .map_err(|e| format!("move partial {install_dir:?} aside: {e}"))?;
            let _ = tokio::fs::remove_dir_all(&trash).await;
        }
        let scratch = std::env::temp_dir()
            .join(format!("objectiveai-pg-scratch-{}", std::process::id()));
        // The scratch dir must exist BEFORE `pg.setup()`: it writes
        // the throwaway initdb's `--pwfile` into it without creating
        // parent directories, and dies with os error 3 otherwise.
        tokio::fs::create_dir_all(&scratch)
            .await
            .map_err(|e| format!("mkdir scratch {scratch:?}: {e}"))?;
        // A couple of retries paper over transient Windows
        // interference (antivirus briefly pinning just-extracted
        // files) without masking real failures.
        let mut result = extract_install(env, &install_dir, &scratch).await;
        for _ in 0..2 {
            if result.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            result = extract_install(env, &install_dir, &scratch).await;
        }
        let _ = tokio::fs::remove_dir_all(&scratch).await;
        result?;
        tokio::fs::write(&marker, b"")
            .await
            .map_err(|e| format!("write {marker:?}: {e}"))?;
    }
    // 5. Explicitly release — dropping a LockClaim deliberately does
    //    NOT release it.
    claim
        .release()
        .map_err(|e| format!("release(db install lock): {e}"))
}

/// Extract the bundled archive into `install_dir` by running
/// `pg.setup()` with a scratch data dir (extract + a throwaway
/// initdb that is deleted by the caller).
async fn extract_install(
    env: &Args,
    install_dir: &Path,
    scratch: &Path,
) -> Result<(), String> {
    let mut settings = postgresql_embedded::Settings::default();
    settings.installation_dir = PathBuf::from(install_dir);
    // The throwaway data dir is a SUBDIRECTORY of scratch and the
    // password file its sibling: `pg.setup()` writes the pwfile
    // before running initdb, and initdb refuses a non-empty data
    // directory (initdb itself creates the missing subdir).
    settings.data_dir = scratch.join("data");
    settings.password_file = scratch.join(".pgpass-scratch");
    settings.temporary = true;
    settings.password = env.pg_password.clone();
    settings.timeout = Some(Duration::from_secs(180));
    let mut pg = postgresql_embedded::PostgreSQL::new(settings);
    pg.setup().await.map_err(|e| format!("install: {e}"))
}

/// Phase 2: ensure this state's cluster exists at
/// `<state_dir>/db/`. Runs inside the caller's `db-init` gate, so
/// at most one initializer works at a time.
///
/// Commit-gated against termination by a SEPARATE marker file
/// (`<state_dir>/db.ready`), NOT by the data dir's mere existence:
/// initdb runs DIRECTLY into `db/`, and the marker is written only
/// after it fully succeeds. `db.ready` present ⇔ a COMPLETE cluster.
/// A supervisor killed mid-initdb leaves a markerless (partial) `db/`,
/// which the next run wipes and redoes rather than serving a broken
/// cluster forever. This is the same marker contract `ensure_installed`
/// uses for the pg-bin extract (`.objectiveai-complete`).
///
/// Why a marker instead of the old staging-dir + `rename` to commit:
/// initdb writes hundreds of files, and on Windows a freshly-written
/// tree is transiently held open by the real-time virus scanner and by
/// the OS's lazy release of initdb's just-exited child processes.
/// Renaming a directory while ANY handle into it is open fails with
/// `Access is denied` (os error 5) — a window that 23-way-parallel
/// initdb hits reliably. (`pg` itself holds no handle here: after
/// `setup()` with no `start()` it is just a `Settings` struct, so the
/// old `drop(pg)`-before-rename never actually helped.) Initializing in
/// place removes the rename entirely; the only thing we create after
/// the scan-sensitive window is our own marker, which we write and
/// close ourselves.
async fn ensure_initdb(env: &Args) -> Result<(), String> {
    let state_dir = env.state_dir();
    let data_dir = state_dir.join("db");
    let ready_marker = state_dir.join("db.ready");

    // Complete cluster already committed? Done.
    if tokio::fs::try_exists(&ready_marker).await.unwrap_or(false) {
        return Ok(());
    }

    // A markerless `db/` is a partial initdb from a crashed predecessor
    // (initdb refuses a non-empty target, so it must go). Rename-then-
    // delete, NOT delete-in-place: Windows directory deletion is
    // asynchronous, so a plain `remove_dir_all` returns while the tree
    // is still pending-delete and initdb's recreate fails with
    // ACCESS_DENIED. The rename frees the name instantly; the renamed
    // tree is swept below.
    if tokio::fs::try_exists(&data_dir).await.unwrap_or(false) {
        let trash = state_dir.join(format!("db.trash-{}", std::process::id()));
        tokio::fs::rename(&data_dir, &trash)
            .await
            .map_err(|e| format!("clear partial initdb {data_dir:?}: {e}"))?;
    }

    // Sweep trash dirs left by this and earlier wipe passes. Best-effort:
    // a pending-delete tree is harmless, and the gate guarantees no live
    // sibling owns these.
    if let Ok(mut entries) = tokio::fs::read_dir(&state_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("db.trash-") {
                let _ = tokio::fs::remove_dir_all(entry.path()).await;
            }
        }
    }

    let mut settings = postgresql_embedded::Settings::default();
    settings.installation_dir = env.bin_dir().join("pg-bin");
    settings.data_dir = data_dir.clone();
    // `Settings::default()` puts the password file in
    // `tempfile::tempdir()` (OS temp root). Pin it next to the data
    // dir (NOT inside — initdb refuses a non-empty data directory)
    // so per-state writes stay inside the state dir.
    settings.password_file = state_dir.join(".pgpass");
    settings.temporary = false;
    settings.password = env.pg_password.clone();
    // initdb routinely takes 10-30s on first run.
    settings.timeout = Some(Duration::from_secs(180));
    let mut pg = postgresql_embedded::PostgreSQL::new(settings);
    pg.setup().await.map_err(|e| format!("initdb: {e}"))?;
    drop(pg);

    // Commit: the marker is written ONLY after a fully-successful
    // initdb, so its presence certifies a complete cluster. It lives
    // OUTSIDE `db/` so it never lands in PGDATA.
    tokio::fs::write(&ready_marker, b"")
        .await
        .map_err(|e| format!("commit initdb marker {ready_marker:?}: {e}"))
}

/// Locate `bin/postgres(.exe)` under the shared install.
/// `postgresql_embedded` extracts each release into a VERSIONED
/// subdirectory (`pg-bin/<version>/bin/...`), so the version dir is
/// resolved at runtime instead of hard-coding one; with multiple
/// versions present (a stale install beside a fresh one) the
/// lexically-highest wins.
fn postgres_binary(env: &Args) -> Result<PathBuf, String> {
    let install_dir = env.bin_dir().join("pg-bin");
    let leaf = Path::new("bin")
        .join(if cfg!(windows) { "postgres.exe" } else { "postgres" });
    let entries = std::fs::read_dir(&install_dir)
        .map_err(|e| format!("read {install_dir:?}: {e}"))?;
    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path().join(&leaf))
        .filter(|candidate| candidate.is_file())
        .collect();
    candidates.sort();
    candidates
        .pop()
        .ok_or_else(|| format!("no <version>/bin/postgres under {install_dir:?}"))
}

/// A random free port on loopback. postgres can't bind port 0
/// itself, so randomness is resolved here (bind, read, release —
/// the usual TOCTOU caveat, narrowed by spawning immediately after).
fn free_port() -> Result<u16, String> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| format!("bind free port: {e}"))?;
    listener
        .local_addr()
        .map(|a| a.port())
        .map_err(|e| format!("local_addr: {e}"))
}

/// Spawn `postgres` directly (NOT `pg_ctl start`, which daemonizes)
/// so the postmaster is a true child that lives and dies with this
/// process. The OS-level "die with the parent" leash — Windows
/// kill-on-close job object, Linux `PR_SET_PDEATHSIG`, macOS kqueue
/// guardian — is the shared
/// [`objectiveai_sdk::subprocess_reaper::spawn`] primitive, the same one
/// the cli uses for tools and plugins.
fn spawn_postgres(env: &Args, port: u16) -> Result<tokio::process::Child, String> {
    let data_dir = env.state_dir().join("db");
    let postgres = postgres_binary(env)?;

    let mut cmd = tokio::process::Command::new(&postgres);
    cmd.arg("-D")
        .arg(&data_dir)
        .arg("-p")
        .arg(port.to_string())
        .arg("-h")
        .arg("127.0.0.1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        // TCP only — no unix-domain sockets to place or clean up.
        cmd.arg("-c").arg("unix_socket_directories=");
    }

    // Sets `kill_on_drop` AND the OS leash so the postmaster dies with
    // this supervisor by any means (force-kill included).
    objectiveai_sdk::subprocess_reaper::spawn(&mut cmd)
        .map_err(|e| format!("spawn {postgres:?}: {e}"))
}

/// Poll TCP until postgres accepts connections, failing fast if the
/// child exits during startup.
async fn wait_ready(
    port: u16,
    child: &mut tokio::process::Child,
) -> Result<(), String> {
    let host = "127.0.0.1";
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        if matches!(
            tokio::time::timeout(
                Duration::from_millis(250),
                tokio::net::TcpStream::connect((host, port)),
            )
            .await,
            Ok(Ok(_))
        ) {
            return Ok(());
        }
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("postgres exited during startup: {status}"));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("postgres did not become ready within 180s".to_string());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// `postgresql://postgres:<password>@127.0.0.1:<port>` — the URL
/// clients connect with, published as the state lock's content. The
/// password is percent-encoded.
fn connection_string(env: &Args, port: u16) -> String {
    format!(
        "postgresql://postgres:{}@127.0.0.1:{port}",
        percent_encode(&env.pg_password)
    )
}

/// Percent-encode for the userinfo part of a URL: unreserved
/// characters pass through, everything else becomes `%XX`.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
