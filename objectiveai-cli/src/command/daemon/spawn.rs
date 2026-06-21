//! `daemon spawn` — launcher + resident foreground daemon.
//!
//! Launcher (`foreground` unset/false): the exact lock flow `api spawn`
//! / `viewer spawn` use — `try_read` the lock, re-exec this cli as the
//! foreground daemon if it isn't held, re-check on child exit.
//!
//! Foreground (`foreground:true`): the resident daemon. Under a blocking
//! init gate it acquires the singleton lock, then launches every
//! `daemon: true` plugin via the SHARED plugin executor
//! (`plugins::run::execute`) as `<exec> daemon begin` — so each resident
//! plugin gets the full bidirectional protocol (it can execute nested
//! commands, exactly like `plugins run` and the conduit's `mcp begin`).
//! The plugins are leashed to this process; if any exits, the whole
//! daemon exits (and the OS releases the lock).

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::daemon::spawn::{Request, ResponseItem};
use objectiveai_sdk::cli::command::plugins::run::{Path as RunPath, Request as RunRequest};

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
        // spawn` use — try_read, exec the foreground daemon if not held,
        // re-check on child exit.
        spawn(ctx).await?;
        Ok(Box::pin(futures::stream::once(async move {
            Ok::<ResponseItem, Error>(ResponseItem { ok: true })
        })))
    }
}

/// Ensure the resident daemon is up, returning its published lock
/// content. Mirrors [`crate::command::viewer::spawn::spawn`]: re-execs
/// THIS cli as the foreground daemon via the shared
/// `spawn_until_lock_published` helper.
pub async fn spawn(ctx: &Context) -> Result<String, Error> {
    let lock_dir = ctx.filesystem.state_dir().join("locks");
    let exe = std::env::current_exe().map_err(|e| Error::Spawn("current_exe".into(), e))?;
    crate::spawn::spawn_until_lock_published(
        &exe,
        &lock_dir,
        super::DAEMON_LOCK_KEY,
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

/// Foreground: the resident daemon.
async fn execute_foreground(ctx: &Context) -> Result<ItemStream, Error> {
    let lock_dir = ctx.filesystem.state_dir().join("locks");
    let lock_err = |e: std::io::Error| Error::Lockfile {
        key: super::DAEMON_LOCK_KEY.to_string(),
        source: e,
    };

    // First acquire the init gate (blocking), then the singleton lock.
    let init = objectiveai_sdk::lockfile::wait_acquire(
        &lock_dir,
        super::DAEMON_INIT_LOCK_KEY,
        "initializing",
    )
    .await
    .map_err(lock_err)?;

    let claim = match objectiveai_sdk::lockfile::try_acquire(
        &lock_dir,
        super::DAEMON_LOCK_KEY,
        "ready",
    )
    .await
    {
        // A sibling daemon already holds the lock — bow out.
        None => {
            let _ = init.release();
            return Ok(Box::pin(futures::stream::empty()));
        }
        Some(claim) => claim,
    };
    init.release().map_err(lock_err)?;

    // Launch every daemon plugin under the SHARED plugin executor, run
    // as `<exec> daemon begin`. `plugins::run::execute` spawns it leashed
    // and drives the full nested-command protocol; we consume (drive) its
    // stream below.
    let manifests: Vec<crate::filesystem::plugins::Manifest> = ctx
        .filesystem
        .list_plugins(0, usize::MAX)
        .await
        .into_iter()
        .filter(|m| m.daemon)
        .collect();
    let mut streams = Vec::new();
    for manifest in manifests {
        let request = RunRequest {
            path_type: RunPath::PluginsRun,
            owner: manifest.owner,
            name: manifest.name,
            version: manifest.version,
            args: vec!["daemon".to_string(), "begin".to_string()],
            base: Default::default(),
        };
        let stream = crate::command::plugins::run::execute(ctx, request).await?;
        streams.push(stream);
    }

    let stream = async_stream::stream! {
        // Hold the lock claim for the daemon's whole life: `LockClaim`
        // never releases on drop (the OS reclaims the handles on process
        // exit — exactly the liveness we want).
        let _claim = claim;

        let mut drains = futures::stream::FuturesUnordered::new();
        for plugin_stream in streams {
            drains.push(async move {
                let mut plugin_stream = plugin_stream;
                // Draining the stream DRIVES the plugin's protocol —
                // nested commands run as items are consumed. The stream
                // ends only when the plugin exits.
                while plugin_stream.next().await.is_some() {}
            });
        }

        // Ready: the launcher's handshake (and the lone item a direct
        // `daemon spawn --foreground` watcher would see).
        yield Ok::<ResponseItem, Error>(ResponseItem { ok: true });

        if drains.is_empty() {
            // No daemon plugins: stay resident so the singleton is held.
            std::future::pending::<()>().await;
        } else {
            // Any plugin exiting ends the whole daemon.
            let _ = drains.next().await;
        }
    };
    Ok(Box::pin(stream))
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
