//! `daemon spawn` — launcher + resident foreground daemon.
//!
//! Launcher (`foreground` unset/false): probe the per-state daemon
//! lock; if a daemon already holds it, return readiness, else re-exec
//! this binary as `daemon spawn --dangerous-advanced {"foreground":true}`
//! detached via the shared lock-published spawner and return once the
//! daemon publishes its lock.
//!
//! Foreground (`foreground:true`): become the resident daemon. Under a
//! blocking init gate (so the final lock is published only when fully
//! ready), launch every `daemon: true` plugin as `<exec> daemon begin`
//! (leashed), bind each one's socket, claim the daemon lock, release the
//! gate, emit one readiness item, and serve until any plugin exits — at
//! which point the whole daemon exits (the leash kills the rest; the OS
//! releases the lock on process death).

use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;

use futures::Stream;
use objectiveai_sdk::cli::command::daemon::spawn::{Request, ResponseItem};
use tokio::process::{ChildStdin, Command};
use tokio::sync::Mutex;

use super::socket;
use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let foreground = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.foreground)
        .unwrap_or(false);
    if foreground {
        execute_foreground(ctx).await
    } else {
        // Non-foreground: the exact lock flow `api spawn` / `viewer
        // spawn` use — try_read the lock, exec the (foreground) daemon
        // if it isn't held, re-check the lock if that exec exits first.
        spawn(ctx).await?;
        Ok(Box::pin(futures::stream::once(async move {
            Ok::<ResponseItem, Error>(ResponseItem { ok: true })
        })))
    }
}

/// Ensure the resident daemon is up, returning its published lock
/// content. Mirrors [`crate::command::viewer::spawn::spawn`] exactly:
/// `spawn_until_lock_published` `try_read`s the lock (short-circuit when
/// the daemon is already up), otherwise re-execs THIS cli as the
/// foreground daemon and waits for the lock to be published, re-probing
/// if the child exits first. Shared by `daemon spawn` (non-foreground)
/// and `plugins daemon notify`.
pub async fn spawn(ctx: &Context) -> Result<String, Error> {
    let lock_dir = ctx.filesystem.state_dir().join("locks");
    let exe = std::env::current_exe().map_err(|e| Error::Spawn("current_exe".into(), e))?;
    crate::spawn::spawn_until_lock_published(
        &exe,
        &lock_dir,
        socket::DAEMON_LOCK_KEY,
        |cmd| {
            cmd.arg("daemon")
                .arg("spawn")
                .arg("--dangerous-advanced")
                .arg("{\"foreground\":true}");
            crate::spawn::apply_config_env(cmd, &ctx.config);
        },
    )
    .await
}

struct PluginHandle {
    child: tokio::process::Child,
    stdin: Arc<Mutex<ChildStdin>>,
    listener: socket::Listener,
}

/// Foreground: the resident daemon.
async fn execute_foreground(ctx: &Context) -> Result<ItemStream, Error> {
    let lock_dir = ctx.filesystem.state_dir().join("locks");
    let lock_err = |e: std::io::Error| Error::Lockfile {
        key: socket::DAEMON_LOCK_KEY.to_string(),
        source: e,
    };

    // First acquire the init gate (blocking), then the regular lock.
    // The gate serializes startup so the regular lock's acquisition
    // never races; a loser (regular lock already held) just bows out.
    let init = objectiveai_sdk::lockfile::wait_acquire(
        &lock_dir,
        socket::DAEMON_INIT_LOCK_KEY,
        "initializing",
    )
    .await
    .map_err(lock_err)?;

    let claim = match objectiveai_sdk::lockfile::try_acquire(
        &lock_dir,
        socket::DAEMON_LOCK_KEY,
        "ready",
    )
    .await
    {
        // A sibling daemon already holds the regular lock — bow out.
        None => {
            let _ = init.release();
            return Ok(Box::pin(futures::stream::empty()));
        }
        Some(claim) => claim,
    };

    // We hold the regular lock now; the gate's job is done.
    init.release().map_err(lock_err)?;

    // Launch every daemon plugin + bind its socket, in parallel.
    let manifests: Vec<crate::filesystem::plugins::Manifest> = ctx
        .filesystem
        .list_plugins(0, usize::MAX)
        .await
        .into_iter()
        .filter(|m| m.daemon)
        .collect();
    let setup =
        futures::future::join_all(manifests.into_iter().map(|m| setup_plugin(ctx, m))).await;
    let mut handles: Vec<PluginHandle> = Vec::new();
    for result in setup {
        match result {
            Ok(handle) => handles.push(handle),
            Err(e) => {
                // Release the regular lock so the next attempt re-spawns
                // cleanly rather than colliding with a dead claim.
                let _ = claim.release();
                return Err(e);
            }
        }
    }

    let stream = async_stream::stream! {
        // Hold the lock claim for the daemon's whole life: `LockClaim`
        // never releases on drop (it would leak the handles, which the
        // OS reclaims on process exit — exactly the liveness we want).
        let _claim = claim;

        let mut waits = futures::stream::FuturesUnordered::new();
        for handle in handles {
            let PluginHandle { child, stdin, listener } = handle;
            tokio::spawn(socket::accept_loop(listener, stdin));
            waits.push(async move {
                let mut child = child;
                let _ = child.wait().await;
            });
        }

        // Ready: this is the launcher's handshake AND the lone item a
        // direct `daemon spawn --foreground` watcher would see.
        yield Ok::<ResponseItem, Error>(ResponseItem { ok: true });

        // Serve until ANY plugin exits — then the whole daemon exits.
        use futures::StreamExt;
        if waits.is_empty() {
            // No daemon plugins: nothing to supervise, but still hold
            // the lock and stay resident so the singleton is honoured.
            std::future::pending::<()>().await;
        } else {
            let _ = waits.next().await;
        }
    };
    Ok(Box::pin(stream))
}

/// Resolve, compartment-provision, and spawn one daemon plugin as
/// `<exec> daemon begin` (leashed), drain its stdout/stderr, and bind
/// its socket. Mirrors the `plugins run` spawn setup.
async fn setup_plugin(
    ctx: &Context,
    manifest: crate::filesystem::plugins::Manifest,
) -> Result<PluginHandle, Error> {
    let owner = manifest.owner.clone();
    let name = manifest.name.clone();
    let version = manifest.version.clone();
    let coord = format!("{owner}/{name}/{version}");

    let (exec, cli_dir) = ctx
        .filesystem
        .resolve_plugin(&owner, &name, &version)
        .await
        .ok_or_else(|| Error::PluginNotFound(coord.clone()))?;

    // The plugin's exec vector plus the daemon entrypoint args. First
    // element is the program; CWD is the plugin's `cli/` folder.
    let mut argv = exec;
    argv.push("daemon".to_string());
    argv.push("begin".to_string());
    let mut argv = argv.into_iter();
    let program = argv
        .next()
        .ok_or_else(|| Error::PluginNotFound(format!("{coord} (empty exec)")))?;
    let program = crate::spawn::resolve_program(program, &cli_dir);

    let state_dir = ctx
        .filesystem
        .state_dir()
        .join("plugins")
        .join(&owner)
        .join(&name)
        .join(&version);
    tokio::fs::create_dir_all(&state_dir)
        .await
        .map_err(Error::PluginSpawn)?;

    let postgres_url = crate::db::compartment::ensure(
        ctx.db_handle().await?,
        crate::db::compartment::Kind::Plugin,
        &owner,
        &name,
        &version,
    )
    .await?;

    let mut nested_ctx = ctx.clone();
    nested_ctx.config.plugin_owner = Some(owner.clone());
    nested_ctx.config.plugin_repository = Some(name.clone());
    nested_ctx.config.plugin_version = Some(version.clone());

    let mut cmd = Command::new(&program);
    cmd.args(argv)
        .current_dir(&cli_dir)
        .env("OBJECTIVEAI_STATE_DIR", &state_dir)
        .env("OBJECTIVEAI_BIN_DIR", &cli_dir)
        .env("OBJECTIVEAI_POSTGRES_URL", postgres_url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::spawn::apply_config_env(&mut cmd, &nested_ctx.config);

    let mut child =
        objectiveai_sdk::subprocess_reaper::spawn(&mut cmd).map_err(Error::PluginSpawn)?;
    let stdin = child.stdin.take().expect("stdin was piped");
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    // Drain the plugin's output so its pipes never wedge it; daemon
    // notify only acks the stdin write, so plugin output is not
    // surfaced.
    drain_pipe(stdout);
    drain_pipe(stderr);

    let socket_path = socket::plugin_socket_path(&ctx.filesystem.state_dir(), &owner, &name, &version);
    let listener = socket::bind(&socket_path)
        .map_err(|e| Error::Daemon(format!("bind socket {socket_path:?}: {e}")))?;

    Ok(PluginHandle {
        child,
        stdin: Arc::new(Mutex::new(stdin)),
        listener,
    })
}

/// Read a child pipe to EOF, discarding everything — purely to keep the
/// kernel pipe from filling and blocking the plugin.
fn drain_pipe<R>(mut pipe: R)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 4096];
        loop {
            match pipe.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::daemon::spawn as sdk;
    use objectiveai_sdk::cli::command::daemon::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::daemon::spawn as sdk;
    use objectiveai_sdk::cli::command::daemon::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
