//! Process-lifetime state — [`GlobalContext`].
//!
//! Everything here is IDENTITY-BLIND BY CONSTRUCTION: no field can
//! ever carry per-request identity, which is what makes the memoized
//! db/python singletons, the agent-lock map, the resident hubs, and
//! the leashed-children map safe to share across every request the
//! daemon will ever serve. Anything identity-flavored lives on
//! [`ScopedContext`](crate::context::ScopedContext), constructed fresh
//! per request.
//!
//! Built once in `main.rs` (or by a programmatic embedder) —
//! synchronously and infallibly, no IO — then cloned freely (every
//! shared field is an `Arc`). The service singletons are LAZY: the
//! first `db_handle()` / `python()` call resolves and memoizes in a
//! `tokio::sync::OnceCell` shared across clones; concurrent callers
//! coalesce on one initialization, and a failed init is not cached so
//! the next call retries. Commands that never touch a service never
//! spawn or connect to it.

use std::sync::Arc;

use dashmap::DashMap;
use objectiveai_sdk::Notifier;
use tokio::sync::{OnceCell, broadcast};

use crate::db;
use crate::filesystem;
use crate::http::agent_instance_route::ConversationHub;
use crate::http::agents_routes::ActiveAgents;
use crate::http::laboratories_routes::LaboratoriesHub;
use crate::http::websocket_laboratory::LaboratoryRegistry;
use crate::run::Config;

/// The resident daemon's in-process hubs — the direct replacements for
/// the former unix sockets. Built once at daemon boot in
/// [`crate::command::daemon::spawn`]'s `execute_foreground` and
/// published on the [`GlobalContext`] via
/// [`GlobalContext::set_resident_hubs`], so every in-process producer
/// (the executor tee, the agent registry, the log writer, the conduit,
/// the laboratories commands, the `agents mcp` dispatch) reaches its
/// consumer with a direct method call instead of a socket hop.
///
/// Intended reference cycle: `active` / `conversations` / `labs_hub`
/// each hold a `GlobalContext` clone, and `GlobalContext` holds this
/// bundle via a `OnceLock`. The cycle is bounded to the daemon's
/// forever-lifetime (these hubs already live for the whole process),
/// so it never leaks in practice — the daemon is a per-state
/// singleton.
#[derive(Clone)]
pub(crate) struct ResidentHubs {
    /// `/listen` broadcast sender (former `daemon.sock`).
    pub broadcast: broadcast::Sender<String>,
    /// Live agent-status hub (former `agents.sock`).
    pub active: ActiveAgents,
    /// Live-conversation hub (former `conversation.sock`).
    pub conversations: ConversationHub,
    /// Connected-laboratory registry (former `laboratories.sock`).
    pub laboratories: LaboratoryRegistry,
    /// Local-laboratory change hub (former `laboratories.sock`).
    pub labs_hub: LaboratoriesHub,
    /// Per-`response_id` MCP notifiers (former per-response mcp sockets).
    pub mcp_notifiers: Arc<DashMap<String, Notifier>>,
    /// The `/user` user-requests hub (tracked per-connection
    /// delivery; see `http::user_routes`).
    pub user: crate::http::user_routes::UserHub,
    /// The `/channels` duplex-channels hub (live coordination; the
    /// durable log lives in `db::channels`). See `http::channel_routes`.
    pub channels: crate::http::channel_routes::ChannelHub,
}

/// One leashed resident server: the held [`tokio::process::Child`]
/// (its OS leash + `kill_on_drop` tie the server's life to the
/// daemon's) and the address it announced in its stdout readiness
/// handshake. The laboratory host additionally carries its stdio
/// dial-list channel — dropped with the entry on every kill path,
/// which closes the child's stdin (EOF = the host's graceful-shutdown
/// signal, container-stop included, even on Windows).
pub(crate) struct ResidentChild {
    pub(crate) child: tokio::process::Child,
    pub(crate) address: Option<String>,
    pub(crate) stdio: Option<Arc<LabHostStdio>>,
}

/// The laboratory host's stdin/stdout dial-list channel (see
/// [`objectiveai_sdk::laboratories::daemon::HostStdioRequest`]). ONE
/// mutex over both halves serializes commands, so at most one is ever
/// outstanding and correlation degenerates to "recv until the ack
/// echoing this request's id" — no pending map.
pub(crate) struct LabHostStdio {
    io: tokio::sync::Mutex<(
        tokio::process::ChildStdin,
        tokio::sync::mpsc::UnboundedReceiver<
            objectiveai_sdk::laboratories::daemon::HostStdioAck,
        >,
    )>,
}

impl LabHostStdio {
    pub(crate) fn new(
        stdin: tokio::process::ChildStdin,
        acks: tokio::sync::mpsc::UnboundedReceiver<
            objectiveai_sdk::laboratories::daemon::HostStdioAck,
        >,
    ) -> Self {
        Self {
            io: tokio::sync::Mutex::new((stdin, acks)),
        }
    }

    /// Send one dial-list command (wrapped in a fresh random request
    /// id) and await the ack echoing that id — the host applied the
    /// mutation (NOT connectivity; dialing retries forever). NO
    /// timeout by design: the host acks every parsed line, and a dead
    /// host closes its pipes, which ends the ack stream and errors
    /// here — so this waits exactly as long as the host is alive and
    /// busy. Errors mean the channel is broken (write failed, ack
    /// stream closed).
    pub(crate) async fn send_host_stdio(
        &self,
        command: &objectiveai_sdk::laboratories::daemon::HostStdioCommand,
    ) -> Result<(), crate::error::Error> {
        use tokio::io::AsyncWriteExt;
        let request = objectiveai_sdk::laboratories::daemon::HostStdioRequest {
            id: uuid::Uuid::new_v4().to_string(),
            command: command.clone(),
        };
        let mut io = self.io.lock().await;
        let (stdin, acks) = &mut *io;
        let mut line = serde_json::to_string(&request)
            .expect("HostStdioRequest serializes");
        line.push('\n');
        stdin.write_all(line.as_bytes()).await.map_err(|e| {
            crate::error::Error::Laboratory(format!(
                "laboratory host stdin write failed: {e}"
            ))
        })?;
        stdin.flush().await.map_err(|e| {
            crate::error::Error::Laboratory(format!(
                "laboratory host stdin flush failed: {e}"
            ))
        })?;
        loop {
            let ack = acks.recv().await.ok_or_else(|| {
                crate::error::Error::Laboratory(
                    "laboratory host stdio channel closed".to_string(),
                )
            })?;
            // A non-matching id is a stale ack from an abandoned
            // predecessor — skip it and keep reading.
            if ack.id == request.id {
                return Ok(());
            }
        }
    }
}

/// One cached db handle plus how it was built — the validity signal
/// for [`GlobalContext::db_handle`]'s fast path.
struct CachedDb {
    handle: db::DbHandle,
    /// Built via the local spawn path (no `db.address` configured):
    /// served only while the "db" resident child is alive, so a
    /// crashed/killed local db rebuilds (respawns) on the next call.
    local: bool,
}

#[derive(Clone)]
pub struct GlobalContext {
    /// When true, `config set` commands are refused. Boot env
    /// (`CONFIG_SET_FORBIDDEN`); stamped onto child processes by
    /// [`crate::spawn::apply_config_env`].
    pub config_set_forbidden: bool,
    /// Raw layout-root override (`OBJECTIVEAI_DIR`) as it arrived from
    /// the environment — kept for the child-process env round-trip
    /// (`apply_config_env`); path work goes through the filesystem
    /// clients instead.
    pub objectiveai_dir: Option<String>,
    /// Raw state-name override (`OBJECTIVEAI_STATE`) — same round-trip
    /// role as `objectiveai_dir`.
    pub objectiveai_state: Option<String>,
    /// Commit-author BOOT DEFAULTS (raw env values). The live values
    /// belong to the per-request scope's filesystem, which re-resolves
    /// them from the on-disk config at scope construction — these are
    /// only the env round-trip for spawned children and the fallback
    /// seed.
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    /// Bind address for the resident daemon's HTTP server (bare
    /// `ADDRESS`); default `127.0.0.1`.
    pub daemon_bind_address: String,
    /// Bind port for the resident daemon's HTTP server (bare `PORT`);
    /// default `0` (OS-assigned).
    pub daemon_bind_port: u16,
    /// Optional shared secret for the daemon's HTTP server (bare
    /// `SECRET`) — the raw BOOT value, kept for the child-env
    /// round-trip (`apply_daemon_env`) and as [`Self::auth_secret`]'s
    /// construction seed. Live verification reads the cell, not this.
    pub daemon_secret: Option<String>,
    /// Optional PRE-DERIVED client signature for this daemon's HTTP
    /// server (bare `SIGNATURE`) — what the daemon hands to the
    /// clients it spawns (viewer, laboratory host). When unset it is
    /// derived from `SECRET`; see [`Self::client_signature`].
    pub daemon_signature: Option<String>,
    /// The LIVE auth secret — what [`crate::http::daemon_auth`]
    /// verifies incoming signatures against RIGHT NOW, and what
    /// [`Self::client_signature`] derives from. Seeded from the bare
    /// `SECRET` env at construction ([`Self::daemon_secret`] keeps the
    /// raw boot value for the child-env round-trip); re-pointed at
    /// daemon boot and on every `daemon config` mutation via
    /// [`Self::apply_daemon_config_to_auth`] — but ONLY by a section
    /// whose `address` is `None` (a `Some` address describes some
    /// OTHER daemon's coordinates; its pair is not ours to verify
    /// against). Shared across clones so the HTTP routes see every
    /// update.
    auth_secret: Arc<std::sync::RwLock<Option<Arc<String>>>>,
    /// Boot-value filesystem client, PRIVATE by design: it serves the
    /// memoized singleton inits (`db_handle` / `python`) and the
    /// identity-blind api/db spawn flows ONLY. Those cells take the
    /// FIRST caller's config view and serve everyone, so a per-scope
    /// filesystem there would advertise influence that cannot exist.
    /// All per-request work uses `ScopedContext::filesystem` instead.
    filesystem: filesystem::Client,
    /// The ONE `reqwest` connection pool every per-scope `HttpClient`
    /// wraps — scopes differ only in identity headers, so they share
    /// the process's TCP/TLS pool instead of re-handshaking per scope.
    http: reqwest::Client,
    /// The daemon's published `http://` connect URL, stored by `run`'s
    /// producer tee right after it ensures the daemon is up. Empty when
    /// the daemon couldn't be spawned. Shared across clones; first set
    /// wins.
    daemon_address: Arc<std::sync::OnceLock<String>>,
    /// Lazily-connected db handle (pool + admin coordinates) — an
    /// INVALIDATABLE cache, not a memo-forever cell: `db config` set
    /// commands kill the resident db and clear this slot
    /// ([`Self::invalidate_db`]), and the next [`Self::db_handle`]
    /// rebuilds it from the then-current config. See the locking
    /// order on [`Self::db_init_gate`].
    db: Arc<tokio::sync::RwLock<Option<CachedDb>>>,
    /// Serializes db-cache REBUILDS and INVALIDATIONS: every rebuild
    /// (including the whole db spawn it may perform) and every
    /// invalidation runs under this mutex, so a kill can never race a
    /// child mid-birth or a handle mid-store. Locking order:
    /// `db_init_gate` → `spawn_gate("db")` → the db slot RwLock →
    /// the `resident_children` DashMap (sync leaf) — never reversed,
    /// and `pool.close()` is never awaited under any of them.
    db_init_gate: Arc<tokio::sync::Mutex<()>>,
    /// Bumped on every db invalidation. The resident LISTEN watchers
    /// hold receivers and drop their (possibly still-healthy, e.g.
    /// outgoing-remote) listener connections to re-resolve the pool —
    /// without this a switch AWAY from a healthy remote would leave
    /// them parked in `recv()` forever.
    db_epoch: Arc<tokio::sync::watch::Sender<u64>>,
    /// Lazily-initialized WASI python runtime — see [`Self::python`].
    python: Arc<OnceCell<crate::python::Python>>,
    /// Per-key in-process gate for agent locks (AIH + tag), shared
    /// across clones. This map IS the whole lock layer: agents run as
    /// in-process daemon tasks, so the former cross-process lockfile
    /// layer is gone — nothing is written to disk. Acquired/released
    /// only through
    /// [`crate::command::agents::locks::{try_acquire, wait_acquire}`].
    agent_locks: Arc<crate::command::agents::locks::AgentLockMap>,
    /// The resident daemon's in-process hubs, published once at daemon
    /// boot (`execute_foreground`). `None` in any process that is not
    /// the resident daemon. Shared across clones (first set wins).
    resident_hubs: Arc<std::sync::OnceLock<ResidentHubs>>,
    /// The persistent server subprocesses the resident daemon spawns —
    /// `db` / `api` / `mcp` / `viewer` / `laboratories` — held here for
    /// the daemon's whole life. They are LEASHED
    /// ([`objectiveai_sdk::subprocess_reaper`]): the OS kills each one
    /// when the daemon dies, and holding the [`tokio::process::Child`]
    /// here keeps it alive meanwhile. The cached `address` is the
    /// server's stdout readiness-handshake coordinate
    /// ([`objectiveai_sdk::process::ServerReady`]) — this map IS the
    /// discovery state.
    resident_children: Arc<DashMap<String, ResidentChild>>,
    /// Per-key spawn serialization for [`Self::resident_children`]:
    /// two concurrent `db_handle()` calls must not both spawn a db.
    /// Clone the inner `Arc` out before locking — never hold a map
    /// guard across an await.
    spawn_gates: Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl GlobalContext {
    /// Sync and IO-free: fold the non-identity half of the boot
    /// [`Config`] in and arm the lazy cells. The identity half seeds
    /// [`ScopedContext::boot`](crate::context::ScopedContext::boot).
    pub fn new(config: &Config) -> Self {
        let filesystem = filesystem::Client::new(
            config.objectiveai_dir.clone(),
            config.objectiveai_state.clone(),
            config.commit_author_name.clone(),
            config.commit_author_email.clone(),
        );
        Self {
            config_set_forbidden: config.config_set_forbidden,
            objectiveai_dir: config.objectiveai_dir.clone(),
            objectiveai_state: config.objectiveai_state.clone(),
            commit_author_name: config.commit_author_name.clone(),
            commit_author_email: config.commit_author_email.clone(),
            daemon_bind_address: config.daemon_address.clone(),
            daemon_bind_port: config.daemon_port,
            daemon_secret: config.daemon_secret.clone(),
            daemon_signature: config.daemon_signature.clone(),
            auth_secret: Arc::new(std::sync::RwLock::new(
                config.daemon_secret.clone().map(Arc::new),
            )),
            filesystem,
            http: reqwest::Client::new(),
            daemon_address: Arc::new(std::sync::OnceLock::new()),
            db: Arc::new(tokio::sync::RwLock::new(None)),
            db_init_gate: Arc::new(tokio::sync::Mutex::new(())),
            db_epoch: Arc::new(tokio::sync::watch::channel(0).0),
            python: Arc::new(OnceCell::new()),
            agent_locks: Arc::new(crate::command::agents::locks::AgentLockMap::new()),
            resident_hubs: Arc::new(std::sync::OnceLock::new()),
            resident_children: Arc::new(DashMap::new()),
            spawn_gates: Arc::new(DashMap::new()),
        }
    }

    /// The boot-value filesystem — singleton inits and identity-blind
    /// spawn flows only (see the field doc). Per-request work uses the
    /// scope's filesystem.
    pub(crate) fn boot_filesystem(&self) -> &filesystem::Client {
        &self.filesystem
    }

    /// The process-wide `reqwest` pool per-scope `HttpClient`s wrap.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// The secret the daemon's auth verifies against RIGHT NOW —
    /// cloned out of the live cell, so a config rotation mid-request
    /// never tears a check.
    pub(crate) fn auth_secret(&self) -> Option<Arc<String>> {
        self.auth_secret
            .read()
            .expect("auth_secret lock poisoned")
            .clone()
    }

    /// Fold a just-read/just-written `daemon` config section into live
    /// auth. ONLY a section claiming THIS daemon — `address: None` —
    /// re-points the secret (to the section's secret, `None` included:
    /// full-replace semantics make `{address: None, secret: None}` an
    /// explicitly OPEN daemon). `address: Some` means the section
    /// describes some other daemon's coordinates, so its secret and
    /// signature are IGNORED; a missing section keeps the current
    /// secret (nothing was stated). The stored SIGNATURE is never read
    /// here — verification always derives from the secret (the
    /// `verify_signature` math in [`crate::http::daemon_auth`]);
    /// trusting a stored signature by equality would make the secret
    /// pointless.
    pub(crate) fn apply_daemon_config_to_auth(
        &self,
        section: Option<&crate::filesystem::config::DaemonConfig>,
    ) {
        let Some(section) = section else {
            return;
        };
        if section.address.is_some() {
            return;
        }
        *self
            .auth_secret
            .write()
            .expect("auth_secret lock poisoned") = section.secret.clone().map(Arc::new);
    }

    /// The signature clients of THIS daemon should present: derived
    /// one-way from the LIVE auth secret when one is set (what the
    /// auth check actually validates — see [`Self::auth_secret`]),
    /// else the bare pre-derived `SIGNATURE` env (a spawner may know
    /// the signature without the secret), else `None` (open server —
    /// no auth to present).
    pub fn client_signature(&self) -> Option<String> {
        if let Some(secret) = self.auth_secret() {
            return Some(crate::http::daemon_auth::derive_signature(&secret));
        }
        self.daemon_signature.clone()
    }

    /// The per-key spawn gate — see [`Self::resident_children`]. The
    /// `Arc` is cloned OUT of the map so the caller locks it with no
    /// map guard held.
    pub(crate) fn spawn_gate(&self, key: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.spawn_gates
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Park a freshly-spawned leashed server child (+ its readiness
    /// address, + the laboratory host's stdio channel) for the
    /// daemon's life. Only the spawn path calls this, under the key's
    /// spawn gate, after confirming no live child — so a live server
    /// is never displaced.
    pub(crate) fn hold_resident_child(
        &self,
        key: &str,
        child: tokio::process::Child,
        address: Option<String>,
        stdio: Option<Arc<LabHostStdio>>,
    ) {
        self.resident_children.insert(
            key.to_string(),
            ResidentChild {
                child,
                address,
                stdio,
            },
        );
    }

    /// The LIVE laboratory host's stdio dial-list channel. `None` when
    /// no host child is running (liveness via the same `try_wait`
    /// probe as [`Self::resident_child_address`]) — callers then fall
    /// back to write-only config semantics; the next spawn seeds from
    /// config.
    pub(crate) fn lab_host_stdio(&self) -> Option<Arc<LabHostStdio>> {
        let mut entry = self.resident_children.get_mut("laboratories")?;
        match entry.child.try_wait() {
            Ok(None) => entry.stdio.clone(),
            _ => {
                drop(entry);
                self.resident_children.remove("laboratories");
                None
            }
        }
    }

    /// The cached readiness address of a LIVE resident child. `None`
    /// when the key has no child, the child has exited (the dead entry
    /// is removed on observation), or the server reported no address.
    /// The liveness probe is `try_wait` — sync, no reaping race (the
    /// child is exclusively ours).
    pub(crate) fn resident_child_address(&self, key: &str) -> Option<Option<String>> {
        let mut entry = self.resident_children.get_mut(key)?;
        match entry.child.try_wait() {
            Ok(None) => Some(entry.address.clone()),
            // Exited (or errored — treat as gone): drop the corpse so
            // the caller respawns.
            _ => {
                drop(entry);
                self.resident_children.remove(key);
                None
            }
        }
    }

    /// Whether the key currently holds a live resident child.
    pub(crate) fn server_child_alive(&self, key: &str) -> bool {
        self.resident_child_address(key).is_some()
    }

    /// Take the resident child out entirely (the kill commands own it
    /// from here — killing, waiting, reporting).
    pub(crate) fn take_resident_child(&self, key: &str) -> Option<tokio::process::Child> {
        self.resident_children.remove(key).map(|(_, rc)| rc.child)
    }

    /// Record the daemon's published `http://` connect URL. Called by
    /// `run`'s producer tee once the daemon is confirmed up. First set
    /// wins; later calls are no-ops.
    pub fn set_daemon_address(&self, url: String) {
        let _ = self.daemon_address.set(url);
    }

    /// The daemon's published `http://` connect URL, when `run`'s
    /// producer tee successfully ensured the daemon this run. `None`
    /// means the daemon couldn't be spawned (or this context never
    /// went through `run`).
    pub fn daemon_address(&self) -> Option<&str> {
        self.daemon_address.get().map(String::as_str)
    }

    /// Publish the resident daemon's in-process hubs. Called once by
    /// `execute_foreground` at daemon boot; first set wins.
    pub(crate) fn set_resident_hubs(&self, hubs: ResidentHubs) {
        let _ = self.resident_hubs.set(hubs);
    }

    /// The resident daemon's in-process hubs, when this context belongs
    /// to the resident daemon process (`None` otherwise). The direct
    /// in-process replacement for the former unix sockets.
    pub(crate) fn resident_hubs(&self) -> Option<&ResidentHubs> {
        self.resident_hubs.get()
    }

    /// The WASI python runtime, initialized on first use and
    /// memoized. First use machine-wide JIT-compiles the embedded
    /// interpreter and publishes `<bin>/cache/rustpython-<hash>.cwasm`
    /// under the bin lock; later uses deserialize that artifact in
    /// milliseconds. Commands that never execute python never pay
    /// either cost.
    pub async fn python(&self) -> Result<&crate::python::Python, crate::error::Error> {
        self.python
            .get_or_try_init(|| crate::python::Python::initialize(self.filesystem.bin_dir()))
            .await
    }

    /// The per-key in-process gate for agent locks — for direct acquire sites
    /// (`crate::command::agents::locks::{try_acquire, wait_acquire}`).
    pub fn agent_locks(&self) -> &crate::command::agents::locks::AgentLockMap {
        &self.agent_locks
    }

    /// A clone of the shared agent-lock map's `Arc` — for the
    /// `AgentInstanceRegistry`, which holds it for its lifetime.
    pub fn agent_locks_arc(&self) -> Arc<crate::command::agents::locks::AgentLockMap> {
        self.agent_locks.clone()
    }

    /// The db pool — the pool-only view of [`Self::db_handle`].
    /// Owned: `Pool` is an Arc-backed clone.
    pub async fn db_client(&self) -> Result<db::Pool, crate::error::Error> {
        Ok(self.db_handle().await?.pool)
    }

    /// The gate serializing db-cache rebuilds and invalidations —
    /// cloned OUT so the caller locks it with no field borrow held
    /// (same pattern as [`Self::spawn_gate`]).
    pub(crate) fn db_init_gate(&self) -> Arc<tokio::sync::Mutex<()>> {
        self.db_init_gate.clone()
    }

    /// A receiver on the db-invalidation epoch — resident LISTEN
    /// watchers select on it to drop their listener and re-resolve
    /// the pool whenever the db cache is invalidated.
    pub(crate) fn db_epoch_rx(&self) -> tokio::sync::watch::Receiver<u64> {
        self.db_epoch.subscribe()
    }

    /// Clear the cached db handle and background-close its pool, then
    /// bump the db epoch. PRECONDITION (documented, not enforced): the
    /// caller holds [`Self::db_init_gate`] — a tokio `Mutex` is not
    /// reentrant, so this must not lock the gate itself. The pool
    /// close is SPAWNED, never awaited here: a healthy outgoing
    /// remote pool has listener connections parked in `recv()`, and
    /// an inline `close().await` would wedge the gate forever;
    /// spawning marks the pool closed on first poll (waiters get
    /// `PoolClosed`) and reaps stragglers in the background.
    pub(crate) async fn invalidate_db(&self) {
        let taken = self.db.write().await.take();
        if let Some(cached) = taken {
            let pool = cached.handle.pool;
            tokio::spawn(async move { pool.close().await });
        }
        self.db_epoch.send_modify(|e| *e += 1);
    }

    /// The db handle — pool plus the admin coordinates it was built
    /// from (address, admin user/password, database name), which
    /// [`crate::db::compartment`] needs to mint derived per-plugin
    /// connection strings. Connected on first use (ensuring the
    /// application database + schema exist) and CACHED until a
    /// `db config` mutation invalidates it — a config change kills
    /// the resident db and clears this cache, and the next call
    /// rebuilds from the then-current config, so db config changes
    /// never require a daemon restart. Identity-blind by design
    /// (reads the boot filesystem — since config is state-only it is
    /// the same file every scope reads). Must NOT connect eagerly:
    /// commands like `db config ...` have to work before any
    /// database exists — they're how you bring one up in the first
    /// place.
    ///
    /// Validity: a LOCAL handle (built via the spawn path) is only
    /// served while the "db" resident child is alive — a crashed or
    /// killed local db is observed on the next call and rebuilt
    /// (respawned). A REMOTE handle (db.address configured) is served
    /// until explicitly invalidated.
    ///
    /// URL resolution: when `db.address` is set in the config, a
    /// remote-postgres URL is composed from the `config db` parts;
    /// otherwise the internal db spawn flow returns the local
    /// cluster's announced `postgresql://` URL (starting the
    /// objectiveai-db supervisor if needed), whose admin coordinates
    /// are parsed back out of that URL (our own
    /// `postgresql://postgres:{password}@{host}:{port}` shape).
    pub async fn db_handle(&self) -> Result<db::DbHandle, crate::error::Error> {
        // Fast path: a valid cached handle, under the read lock only.
        {
            let slot = self.db.read().await;
            if let Some(cached) = &*slot {
                if !cached.local || self.server_child_alive("db") {
                    return Ok(cached.handle.clone());
                }
            }
        }
        // Slow path: rebuild under the init gate (concurrent callers
        // coalesce; a failed build caches nothing so the next call
        // retries).
        let gate = self.db_init_gate();
        let _guard = gate.lock().await;
        // Double-check: someone rebuilt while we waited. A stale
        // local-dead entry is taken and its pool close backgrounded.
        {
            let mut slot = self.db.write().await;
            match &*slot {
                Some(cached) if !cached.local || self.server_child_alive("db") => {
                    return Ok(cached.handle.clone());
                }
                Some(_) => {
                    if let Some(stale) = slot.take() {
                        let pool = stale.handle.pool;
                        tokio::spawn(async move { pool.close().await });
                    }
                }
                None => {}
            }
        }
        let mut config = self.filesystem.read_config().await?;
        let address = config.db().get_address().map(String::from);
        let local = address.is_none();
        let (url, address, admin_user, admin_password) = match address {
            Some(address) => {
                let db = config.db();
                let user = db
                    .get_user()
                    .unwrap_or(crate::filesystem::config::DB_DEFAULT_USER)
                    .to_string();
                let password = db
                    .get_password()
                    .unwrap_or(crate::filesystem::config::DB_DEFAULT_PASSWORD)
                    .to_string();
                let url = db::config_url(&address, &user, &password);
                (url, address, user, password)
            }
            None => {
                let url = crate::command::db::spawn::spawn(self).await?;
                let (address, user, password) =
                    parse_spawn_db_url(&url).ok_or_else(|| {
                        crate::error::Error::Instance(format!(
                            "the db announced an unparseable URL: {url}"
                        ))
                    })?;
                (url, address, user, password)
            }
        };
        let database = config
            .db()
            .get_database()
            .unwrap_or(crate::filesystem::config::DB_DEFAULT_DATABASE)
            .to_string();
        let pool = db::init(&url, &database).await?;
        let handle = db::DbHandle {
            pool,
            address,
            admin_user,
            admin_password,
            database,
        };
        *self.db.write().await = Some(CachedDb {
            handle: handle.clone(),
            local,
        });
        Ok(handle)
    }
}

// NOTE: the daemon deliberately has NO MCP-timeout resolver for its
// own MCP clients — it never bounds its own MCP calls (connect +
// per-call timeouts are `None`; it waits forever). The user's
// `api.mcp_call_timeout_ms` config is consumed by the scope's
// `build_http_client` as the per-request `X-MCP-CALL-TIMEOUT` header;
// `api.mcp_connect_timeout_ms` is projected onto a spawned API's
// `MCP_CONNECT_TIMEOUT` env (see `resolve_mcp_connect_timeout_ms_opt`).

/// The configured `api.mcp_connect_timeout_ms`, or `None` when unset
/// — the api spawn projects it onto the spawned server's
/// `MCP_CONNECT_TIMEOUT` env ONLY when the user explicitly set it,
/// so an unset knob lets the api resolve its own default. The `fs`
/// parameter names WHOSE config view is read: the boot filesystem for
/// the identity-blind api spawn, a scope's filesystem for handler
/// flows.
pub async fn resolve_mcp_connect_timeout_ms_opt(
    fs: &filesystem::Client,
) -> Result<Option<u64>, crate::error::Error> {
    let mut config = fs
        .read_config()
        .await?;
    Ok(config.api().get_mcp_connect_timeout_ms())
}

/// Effective backoff max-elapsed-time (ms) — the retry budget for the
/// daemon's own MCP client. The merged `api.backoff_max_elapsed_time_ms`
/// config value, or the canonical default (60000ms) when unset. The
/// other exponential-backoff knobs keep their built-in defaults.
pub async fn resolve_backoff_max_elapsed_time_ms(
    fs: &filesystem::Client,
) -> Result<u64, crate::error::Error> {
    Ok(resolve_backoff_max_elapsed_time_ms_opt(fs).await?.unwrap_or(60000))
}

/// The configured `api.backoff_max_elapsed_time_ms`, or `None` when
/// unset — the api spawn uses this to project the backoff env onto the
/// spawned server only when the user explicitly set it.
pub async fn resolve_backoff_max_elapsed_time_ms_opt(
    fs: &filesystem::Client,
) -> Result<Option<u64>, crate::error::Error> {
    let mut config = fs
        .read_config()
        .await?;
    Ok(config.api().get_backoff_max_elapsed_time_ms())
}

/// Parse `(host:port, user, password)` out of the db's announced URL.
/// The format is OUR OWN — `objectiveai-db` announces exactly
/// `postgresql://{user}:{percent-encoded password}@{host}:{port}`
/// (see its `connection_string`) — so this is a structural split, not
/// a general URL parser. `None` on any shape mismatch.
fn parse_spawn_db_url(url: &str) -> Option<(String, String, String)> {
    let rest = url
        .strip_prefix("postgresql://")
        .or_else(|| url.strip_prefix("postgres://"))?;
    let (credentials, address) = rest.rsplit_once('@')?;
    let (user, encoded_password) = credentials.split_once(':')?;
    let password = percent_encoding::percent_decode_str(encoded_password)
        .decode_utf8()
        .ok()?
        .into_owned();
    Some((address.to_string(), user.to_string(), password))
}
