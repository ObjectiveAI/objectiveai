//! `daemon spawn` — launcher + resident foreground daemon.
//!
//! Launcher (`foreground` unset/false): the lock-published spawn flow
//! — `try_read` the lock, re-exec this binary as the foreground daemon
//! if it isn't held, re-check on child exit. (The daemon is the LAST
//! lock-discovered process; its servers are leashed children with a
//! stdout ready handshake instead.)
//!
//! Foreground (`foreground:true`): the resident daemon. Under a blocking
//! init gate it binds the HTTP listener and acquires the
//! singleton lock (publishing the client-connect `http://` URL as the lock
//! content, like `objectiveai-api` publishes its `http://` URL), brings
//! up the [`crate::http::daemon_stream`] hub (`/listen` broadcast SSE +
//! `/execute` POST→SSE runner + fixed-name producer socket), then launches
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

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let foreground = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.foreground)
        .unwrap_or(false);
    if foreground {
        execute_foreground(global, scoped).await
    } else {
        // Non-foreground: the lock-published spawn flow — try_read,
        // exec the foreground daemon if not held, re-check on child
        // exit.
        spawn(global, scoped).await?;
        Ok(Box::pin(futures::stream::once(async move {
            Ok::<ResponseItem, Error>(ResponseItem { ok: true })
        })))
    }
}

/// Ensure the resident daemon is up, returning its published lock
/// content. Mirrors [`crate::command::viewer::spawn::spawn`]: re-execs
/// THIS cli as the foreground daemon via the shared
/// `spawn_until_lock_published` helper.
pub async fn spawn(global: &GlobalContext, scoped: &ScopedContext) -> Result<String, Error> {
    let lock_dir = scoped.filesystem.state_dir().join("locks");
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
            crate::spawn::apply_config_env(cmd, global, scoped);
            // The foreground daemon reads its bind config as bare
            // `ADDRESS`/`PORT`/`SECRET`; stamp them here (never for
            // plugins/tools).
            crate::spawn::apply_daemon_env(cmd, global);
            // The resident daemon is a per-state singleton service, not
            // part of any agent's lineage. Since the producer tee makes
            // ANY command auto-spawn it, scrub the transient identity
            // `apply_config_env` just stamped — otherwise whichever
            // command happens to spawn it first leaks its agent/plugin
            // identity into the long-lived daemon (and into everything
            // the daemon itself spawns). The daemon then boots with the
            // defaults (`agent_instance_hierarchy` = "daemon", the
            // rest unset).
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
        },
    )
    .await
}

/// Foreground: the resident daemon.
async fn execute_foreground(global: &GlobalContext, scoped: &ScopedContext) -> Result<ItemStream, Error> {
    let lock_dir = scoped.filesystem.state_dir().join("locks");
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

    // Bind the HTTP listener BEFORE claiming the
    // singleton, so its real (post-`:0`) address can be published as the
    // lock content. Binding happens under the init gate, which
    // serializes startup — at most one foreground races here at a time.
    let http_listener = match tokio::net::TcpListener::bind((
        global.daemon_bind_address.as_str(),
        global.daemon_bind_port,
    ))
    .await
    {
        Ok(listener) => listener,
        Err(e) => {
            let _ = init.release();
            return Err(Error::Spawn("daemon http bind".into(), e));
        }
    };
    // Build the client-connect URL published in the lock — an `http://`
    // URL (the command channel, the broadcast `/listen`, and the SSE
    // watcher routes are all plain HTTP; only the `/laboratory` host
    // channel is a WebSocket, and its dialer re-derives `ws://` from
    // this address). A wildcard bind (`0.0.0.0` / `::`) maps to loopback
    // so the published address is actually connectable.
    let bound = match http_listener.local_addr() {
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
            format!("http://{}", std::net::SocketAddr::new(connect_ip, addr.port()))
        }
        Err(e) => {
            let _ = init.release();
            return Err(Error::Spawn("daemon http local_addr".into(), e));
        }
    };

    let state_dir = scoped.filesystem.state_dir();

    // Publish the client-connect `http://` URL as the lock content (the
    // `api` / `viewer` spawn convention), so a caller reading the lock
    // discovers exactly where to connect. Published only now that BOTH the
    // HTTP listener and the producer socket are up.
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
            drop(http_listener);
            let _ = init.release();
            return Ok(Box::pin(futures::stream::empty()));
        }
        Some(claim) => claim,
    };
    init.release().map_err(lock_err)?;

    // Bring up the broadcast hub: in-process producers (the run tee)
    // push pre-serialized frames onto this channel via the global
    // context's resident hubs, and they fan out to every connected `/listen` SSE
    // client. The `_rx` clone keeps the channel open for the daemon's
    // whole life.
    let (tx, _rx) = tokio::sync::broadcast::channel::<String>(1024);
    // Fold the persisted `daemon` config section into live auth at
    // boot, the same rule every `daemon config` mutation applies: a
    // section with `address: None` claims THIS daemon and its secret
    // becomes the auth secret (over the bare `SECRET` env seed);
    // `address: Some` or no section leaves the env seed in place.
    let config = scoped.filesystem.read_config().await?;
    global.apply_daemon_config_to_auth(config.daemon.as_ref());
    // The live agent-status hub: its own broadcast of `AgentEvent` frames,
    // fed by AIH-lock announcements on `agents.sock` and watched for
    // release. Held in scope for the daemon's life (its sender clone keeps
    // the channel open).
    let (agents_tx, _agents_rx) = tokio::sync::broadcast::channel::<
        crate::http::agents_routes::StatusChange,
    >(1024);
    let active = crate::http::agents_routes::ActiveAgents::new(
        state_dir.clone(),
        agents_tx,
        global.clone(),
    );
    // The live-conversation hub: log-writer tee frames arriving on
    // `conversation.sock` fan out per-AIH to `/agents/instances/{*aih}`
    // subscribers. Held in scope for the daemon's life.
    let (conversation_tx, _conversation_rx) = tokio::sync::broadcast::channel::<(
        std::sync::Arc<str>,
        std::sync::Arc<str>,
    )>(1024);
    let conversations = crate::http::agent_instance_route::ConversationHub::new(
        conversation_tx,
        global.clone(),
    );
    let laboratories =
        crate::http::websocket_laboratory::LaboratoryRegistry::new();
    // The live-laboratories hub: local-scan cache + coalesced change
    // feed for `/laboratories/list` + `/laboratories/{id}`. Its
    // resident tasks (scanner, registry forwarder, attachments
    // watcher) live for the daemon's life.
    let labs_hub = crate::http::laboratories_routes::LaboratoriesHub::new(
        laboratories.clone(),
        global.clone(),
    );
    labs_hub.spawn_tasks();
    // Publish the in-process hubs on the shared `GlobalContext` so
    // every in-process producer reaches its consumer directly (the
    // former unix sockets). Every `/execute`-derived pair and
    // `DaemonHttpState` shares Arc-siblings of this one, so this
    // single set is visible everywhere. `mcp_notifiers` replaces the
    // per-response mcp sockets.
    let mcp_notifiers = std::sync::Arc::new(dashmap::DashMap::new());
    // The `/user` user-requests hub: pending outbound requests +
    // tracked per-connection delivery. Held here for the daemon's
    // life like every other hub.
    let user = crate::http::user_routes::UserHub::new();
    // The `/channels` duplex-channels hub: live connection + offer
    // coordination (the durable log lives in the DB).
    let channels = crate::http::channel_routes::ChannelHub::new();
    global.set_resident_hubs(crate::context::ResidentHubs {
        broadcast: tx.clone(),
        active: active.clone(),
        conversations: conversations.clone(),
        laboratories: laboratories.clone(),
        labs_hub: labs_hub.clone(),
        mcp_notifiers,
        user: user.clone(),
        channels: channels.clone(),
    });
    crate::http::daemon_stream::serve_http(
        http_listener,
        tx.clone(),
        global.clone(),
        scoped.clone(),
        active.clone(),
        conversations.clone(),
        laboratories.clone(),
        labs_hub.clone(),
        user,
        channels,
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
    let manifests: Vec<crate::filesystem::plugins::Manifest> = scoped
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
        let stream = crate::command::plugins::run::execute(global, scoped, request).await?;
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

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::daemon::spawn as sdk;
    use objectiveai_sdk::cli::command::daemon::spawn::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
