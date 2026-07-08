//! `daemon spawn` — launcher + resident foreground daemon.
//!
//! Launcher (`foreground` unset/false): the exact lock flow `api spawn`
//! / `viewer spawn` use — `try_read` the lock, re-exec this cli as the
//! foreground daemon if it isn't held, re-check on child exit.
//!
//! Foreground (`foreground:true`): the resident daemon. Under a blocking
//! init gate it binds the broadcast WebSocket listener and acquires the
//! singleton lock (publishing the client-connect `ws://` URL as the lock
//! content, like `objectiveai-api` publishes its `http://` URL), brings
//! up the [`crate::websockets::daemon_stream`] hub (`/listen` broadcast +
//! `/execute` in-process runner + fixed-name producer socket), then launches
//! every `daemon: true` plugin via the SHARED plugin executor
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
            // The resident daemon is a per-state singleton service, not
            // part of any agent's lineage. Since the producer tee makes
            // ANY command auto-spawn it, scrub the transient identity
            // `apply_config_env` just stamped — otherwise whichever
            // command happens to spawn it first leaks its agent/plugin
            // identity into the long-lived daemon (and into everything
            // the daemon itself spawns). The daemon then boots with the
            // defaults (`agent_instance_hierarchy` = "cli", the rest
            // unset).
            for var in [
                "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY",
                "OBJECTIVEAI_AGENT_ID",
                "OBJECTIVEAI_AGENT_FULL_ID",
                "OBJECTIVEAI_AGENT_REMOTE",
                "OBJECTIVEAI_RESPONSE_ID",
                "OBJECTIVEAI_RESPONSE_IDS",
                "OBJECTIVEAI_PLUGIN_OWNER",
                "OBJECTIVEAI_PLUGIN_REPOSITORY",
                "OBJECTIVEAI_PLUGIN_VERSION",
            ] {
                cmd.env_remove(var);
            }
            cmd.env_remove(objectiveai_sdk::mcp::MCP_SESSION_ID_ENV);
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

    // Bind the broadcast WebSocket listener BEFORE claiming the
    // singleton, so its real (post-`:0`) address can be published as the
    // lock content. Binding happens under the init gate, which
    // serializes startup — at most one foreground races here at a time.
    let ws_listener = match tokio::net::TcpListener::bind((
        ctx.config.daemon_address.as_str(),
        ctx.config.daemon_port,
    ))
    .await
    {
        Ok(listener) => listener,
        Err(e) => {
            let _ = init.release();
            return Err(Error::Spawn("daemon ws bind".into(), e));
        }
    };
    // Build the client-connect URL published in the lock — a `ws://`
    // URL, mirroring how `objectiveai-api` publishes its `http://` URL:
    // a wildcard bind (`0.0.0.0` / `::`) maps to loopback so the
    // published address is actually connectable.
    let bound = match ws_listener.local_addr() {
        Ok(addr) => {
            let connect_ip = match addr.ip() {
                std::net::IpAddr::V4(v4) if v4.is_unspecified() => {
                    std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
                }
                std::net::IpAddr::V6(v6) if v6.is_unspecified() => {
                    std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
                }
                ip => ip,
            };
            format!("ws://{}", std::net::SocketAddr::new(connect_ip, addr.port()))
        }
        Err(e) => {
            let _ = init.release();
            return Err(Error::Spawn("daemon ws local_addr".into(), e));
        }
    };

    // Bind the producer socket BEFORE publishing the lock, so a held lock
    // guarantees the socket is already listening — a producer then either
    // connects on the first try or the daemon is dead (no connect retry).
    let state_dir = ctx.filesystem.state_dir();
    let socket_listener =
        match crate::websockets::daemon_stream::bind_socket_listener(&state_dir) {
            Ok(listener) => listener,
            Err(e) => {
                drop(ws_listener);
                let _ = init.release();
                return Err(Error::Spawn("daemon socket bind".into(), e));
            }
        };
    // The dedicated agents-status producer socket, bound under the same
    // init gate as the broadcast socket (a held daemon lock ⇒ it is up).
    let agents_socket_listener =
        match crate::websockets::websocket_agents::bind_agents_socket_listener(&state_dir) {
            Ok(listener) => listener,
            Err(e) => {
                drop(ws_listener);
                drop(socket_listener);
                let _ = init.release();
                return Err(Error::Spawn("daemon agents socket bind".into(), e));
            }
        };
    // The dedicated live-conversation producer socket (log-writer tee),
    // same init-gate guarantee.
    let conversation_socket_listener =
        match crate::websockets::websocket_agent_instance::bind_conversation_socket_listener(
            &state_dir,
        ) {
            Ok(listener) => listener,
            Err(e) => {
                drop(ws_listener);
                drop(socket_listener);
                drop(agents_socket_listener);
                let _ = init.release();
                return Err(Error::Spawn("daemon conversation socket bind".into(), e));
            }
        };

    // Publish the client-connect `ws://` URL as the lock content (the
    // `api` / `viewer` spawn convention), so a caller reading the lock
    // discovers exactly where to connect. Published only now that BOTH the
    // WebSocket listener and the producer socket are up.
    let claim = match objectiveai_sdk::lockfile::try_acquire(
        &lock_dir,
        super::DAEMON_LOCK_KEY,
        &bound,
    )
    .await
    {
        // A sibling daemon already holds the lock — drop our listeners and
        // bow out.
        None => {
            drop(ws_listener);
            drop(socket_listener);
            drop(agents_socket_listener);
            drop(conversation_socket_listener);
            let _ = init.release();
            return Ok(Box::pin(futures::stream::empty()));
        }
        Some(claim) => claim,
    };
    init.release().map_err(lock_err)?;

    // Bring up the broadcast hub: producer streams fed into the
    // fixed-name local socket fan out to every connected WebSocket client
    // of the root endpoint. Both listeners share one broadcast channel of
    // pre-serialized frames; the sender clones they hold keep the channel
    // open for the daemon's whole life.
    let (tx, _rx) = tokio::sync::broadcast::channel::<String>(1024);
    // Optional WS auth: when `DAEMON_SECRET` is set, every connection's
    // first-message auth preamble must carry a valid signature; when
    // unset, the server is open (the preamble is consumed regardless).
    let secret = ctx.config.daemon_secret.clone().map(std::sync::Arc::new);
    // The live agent-status hub: its own broadcast of `AgentEvent` frames,
    // fed by AIH-lock announcements on `agents.sock` and watched for
    // release. Held in scope for the daemon's life (its sender clone keeps
    // the channel open).
    let (agents_tx, _agents_rx) = tokio::sync::broadcast::channel::<
        crate::websockets::websocket_agents::StatusChange,
    >(1024);
    let active = crate::websockets::websocket_agents::ActiveAgents::new(
        state_dir.clone(),
        agents_tx,
        ctx.clone(),
    );
    // The live-conversation hub: log-writer tee frames arriving on
    // `conversation.sock` fan out per-AIH to `/agents/instances/{*aih}`
    // subscribers. Held in scope for the daemon's life.
    let (conversation_tx, _conversation_rx) = tokio::sync::broadcast::channel::<(
        std::sync::Arc<str>,
        std::sync::Arc<str>,
    )>(1024);
    let conversations = crate::websockets::websocket_agent_instance::ConversationHub::new(
        conversation_tx,
        ctx.clone(),
    );
    crate::websockets::daemon_stream::serve_ws(
        ws_listener,
        tx.clone(),
        secret,
        ctx.clone(),
        active.clone(),
        conversations.clone(),
    );
    crate::websockets::daemon_stream::serve_socket_listener(socket_listener, tx.clone());
    crate::websockets::websocket_agents::serve_agents_socket_listener(
        agents_socket_listener,
        active.clone(),
    );
    crate::websockets::websocket_agent_instance::serve_conversation_socket_listener(
        conversation_socket_listener,
        conversations.clone(),
    );
    // Best-effort: seed the registry with agents already holding a lock
    // when the daemon started (off the boot path — no DB round-trip block).
    tokio::spawn({
        let active = active.clone();
        async move {
            active.reconcile_startup().await;
        }
    });
    // Live tag tracking: broadcast an `Updated` for an agent whenever its
    // bound tags change (a `tags_changed` NOTIFY from the DB). Resident for
    // the daemon's life; reconnects on listener error.
    tokio::spawn(active.clone().watch_tag_changes());
    // Live laboratory tracking — two independent watchers: the ATTACHED
    // set (attach/detach NOTIFY) and the ACTIVE set (per-spawn-pass
    // replace NOTIFY). Same lifetime + reconnect behavior as the tag
    // watcher above.
    tokio::spawn(active.clone().watch_attachment_changes());
    tokio::spawn(active.clone().watch_active_laboratory_changes());

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
