//! Process-wide state threaded into every bare-naked `command::*::execute`.
//!
//! `Context` is constructed once in `main.rs` (or by a programmatic
//! embedder) — synchronously and infallibly, no IO — then borrowed
//! into the command tree. The three service clients are LAZY: the
//! first `api_client()` / `viewer_client()` / `db_client()` call
//! resolves the server's address (on-disk config, else the matching
//! spawn flow, which itself short-circuits to the lock-published URL
//! when the server is already up), builds the client, and memoizes it
//! in a `tokio::sync::OnceCell` shared across `Context` clones (Arc).
//! Concurrent callers coalesce on one initialization; a failed init
//! is not cached, so the next call retries. Commands that never touch
//! a service never spawn or connect to it.
//!
//! Identity hazard: the memoized API `HttpClient` snapshots
//! `config.agent_instance_hierarchy` + `config.mcp_session_id` into
//! headers. A clone that mutates those fields MUST call
//! [`Context::reset_api_client`] afterwards — see that method's doc.

use std::sync::Arc;

use objectiveai_sdk::HttpClient;
use tokio::sync::OnceCell;

use crate::db;
use crate::filesystem;
use crate::plugin_path::PluginPath;
use crate::run::Config;
use crate::viewer_client::ViewerClient;

#[derive(Clone)]
pub struct Context {
    pub config: Config,
    pub filesystem: filesystem::Client,
    /// The plugin a command is running on behalf of, when it was
    /// invoked through a plugin's nested-command protocol. Assembled
    /// from the `OBJECTIVEAI_PLUGIN_*` env vars (via `Config`).
    pub plugin: Option<PluginPath>,
    /// Lazily-built API `HttpClient` — see [`Context::api_client`].
    api: Arc<OnceCell<HttpClient>>,
    /// Lazily-built viewer client — see [`Context::viewer_client`].
    /// No per-request identity; always shared across clones.
    viewer: Arc<OnceCell<ViewerClient>>,
    /// Lazily-connected db handle (pool + admin coordinates) — see
    /// [`Context::db_handle`]. No per-request identity; always
    /// shared across clones.
    db: Arc<OnceCell<db::DbHandle>>,
    /// Lazily-initialized WASI python runtime — see
    /// [`Context::python`]. No per-request identity; always shared
    /// across clones.
    python: Arc<OnceCell<crate::python::Python>>,
}

impl Context {
    pub fn new(config: Config) -> Self {
        let filesystem = filesystem::Client::new(
            config.objectiveai_dir.clone(),
            config.objectiveai_state.clone(),
            config.commit_author_name.clone(),
            config.commit_author_email.clone(),
        );
        let plugin = PluginPath::from_parts(
            config.plugin_owner.clone(),
            config.plugin_repository.clone(),
            config.plugin_version.clone(),
        );
        Self {
            config,
            filesystem,
            plugin,
            api: Arc::new(OnceCell::new()),
            viewer: Arc::new(OnceCell::new()),
            db: Arc::new(OnceCell::new()),
            python: Arc::new(OnceCell::new()),
        }
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

    /// The API `HttpClient`, built on first use and memoized.
    ///
    /// Address resolution: `api.address` from the merged (`--final`)
    /// config view when set, else the `api spawn` flow — which
    /// returns the lock-published URL of an already-running api
    /// without spawning, or starts one and waits for its lock.
    pub async fn api_client(&self) -> Result<&HttpClient, crate::error::Error> {
        self.api
            .get_or_try_init(|| async {
                let mut config = self
                    .filesystem
                    .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
                    .await?;
                let address = match config.api().get_address() {
                    Some(a) => ensure_scheme(a),
                    None => crate::command::api::spawn::spawn(self).await?,
                };
                Ok(build_http_client(&self.config, &mut config, address))
            })
            .await
    }

    /// Builds an `HttpClient` for direct GitHub reads (agents / swarms /
    /// functions / profiles) — **without** spawning or contacting the
    /// ObjectiveAI API. The `github_*` methods talk to github.com directly
    /// and never use the base `address`, so we hand it a deliberately
    /// invalid one: any accidental API call through this client fails loudly
    /// instead of silently hitting a real server. `x_github_authorization`
    /// (from config) authenticates the GitHub requests. Built fresh per
    /// call — these commands are short-lived and don't memoize a client.
    pub async fn github_http_client(
        &self,
    ) -> Result<HttpClient, crate::error::Error> {
        let mut config = self
            .filesystem
            .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
            .await?;
        // Tripwire address — this client must only be used for github_* calls.
        let address = "https://api.invalid.objectiveai-github-client-only/".to_string();
        Ok(build_http_client(&self.config, &mut config, address))
    }

    /// The MCP [`Backoff`](objectiveai_sdk::mcp::Backoff) bundle this CLI
    /// process uses for ALL of its own MCP clients (the streaming
    /// conduit, through which every CLI MCP connection flows). The
    /// connect + per-call timeouts come from the merged (`--final`)
    /// `api.mcp_timeout_ms` config value (default 60000ms). The backoff
    /// **retry budget** is INDEPENDENT of that timeout: it's read from
    /// the `MCP_BACKOFF_MAX_ELAPSED_TIME` environment variable (the CLI
    /// loads `<OBJECTIVEAI_DIR>/.env` in `main`, the same file the api
    /// reads), defaulting to the SDK default (40s) when unset. The
    /// shared test `.env` sets it to `0` so a genuine MCP failure fails
    /// fast instead of retrying for the full window. The remaining
    /// exponential-backoff knobs keep [`Backoff::default`]'s values.
    /// Callers that need to distinguish "configured" from "unset" (the
    /// api spawn) use [`Self::resolve_mcp_timeout_ms_opt`] instead.
    pub async fn resolve_mcp_backoff(
        &self,
    ) -> Result<objectiveai_sdk::mcp::Backoff, crate::error::Error> {
        let timeout_ms = self.resolve_mcp_timeout_ms_opt().await?.unwrap_or(60000);
        let default = objectiveai_sdk::mcp::Backoff::default();
        let max_elapsed_time_ms = std::env::var("MCP_BACKOFF_MAX_ELAPSED_TIME")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(default.max_elapsed_time_ms);
        Ok(objectiveai_sdk::mcp::Backoff {
            connect_timeout_ms: timeout_ms,
            call_timeout_ms: timeout_ms,
            max_elapsed_time_ms,
            ..default
        })
    }

    /// The configured `api.mcp_timeout_ms`, or `None` when unset —
    /// preserving the distinction [`Self::resolve_mcp_backoff`] erases.
    ///
    /// The api spawn uses this to project the `MCP_*` timeout env onto
    /// the spawned (machine-wide, shared) server ONLY when the user
    /// explicitly set a timeout. Leaving it unset lets the api resolve it
    /// itself — `<OBJECTIVEAI_DIR>/.env`, then its built-in default. That
    /// fallback is behaviorally identical to passing the canonical
    /// default in production, and it's what the test `.env` (which sets
    /// `MCP_BACKOFF_*=0` to fast-fail real MCP errors instead of masking
    /// them with retry storms) relies on: unconditionally setting the
    /// default here would clobber that `0` on the shared api.
    pub async fn resolve_mcp_timeout_ms_opt(
        &self,
    ) -> Result<Option<u64>, crate::error::Error> {
        let mut config = self
            .filesystem
            .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
            .await?;
        Ok(config.api().get_mcp_timeout_ms())
    }

    /// The synchronous-response viewer client, built on first use and
    /// memoized.
    ///
    /// Address resolution mirrors [`Self::api_client`]:
    /// `viewer.address` from the merged config view when set, else
    /// the `viewer spawn` flow. Signature: env `VIEWER_SIGNATURE`,
    /// else `viewer.signature` from the same view.
    pub async fn viewer_client(&self) -> Result<&ViewerClient, crate::error::Error> {
        self.viewer
            .get_or_try_init(|| async {
                let mut config = self
                    .filesystem
                    .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
                    .await?;
                let address = match config.viewer().get_address() {
                    Some(a) => ensure_scheme(a),
                    None => crate::command::viewer::spawn::spawn(self).await?,
                };
                let signature = env("VIEWER_SIGNATURE")
                    .or_else(|| config.viewer().get_signature().map(String::from));
                Ok(ViewerClient::new(address, signature))
            })
            .await
    }

    /// The db pool, connected on first use and memoized — the
    /// pool-only view of [`Self::db_handle`].
    pub async fn db_client(&self) -> Result<&db::Pool, crate::error::Error> {
        Ok(&self.db_handle().await?.pool)
    }

    /// The db handle — pool plus the admin coordinates it was built
    /// from (address, admin user/password, database name), which
    /// [`crate::db::compartment`] needs to mint derived per-plugin
    /// connection strings. Connected on first use (ensuring the
    /// application database + schema exist) and memoized. Must NOT
    /// connect eagerly: commands like `db config ...` and `db spawn`
    /// have to work before any database exists — they're how you
    /// bring one up in the first place.
    ///
    /// URL resolution: when `db.address` is set in the merged config
    /// view, a remote-postgres URL is composed from the `config db`
    /// parts; otherwise the `db spawn` flow returns the local
    /// cluster's lock-published `postgresql://` URL (starting the
    /// objectiveai-db supervisor if needed), whose admin coordinates
    /// are parsed back out of that URL (our own
    /// `postgresql://postgres:{password}@{host}:{port}` shape).
    pub async fn db_handle(&self) -> Result<&db::DbHandle, crate::error::Error> {
        self.db
            .get_or_try_init(|| async {
                let mut config = self
                    .filesystem
                    .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
                    .await?;
                let address = config.db().get_address().map(String::from);
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
                                    "db spawn published an unparseable URL: {url}"
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
                Ok(db::DbHandle {
                    pool,
                    address,
                    admin_user,
                    admin_password,
                    database,
                })
            })
            .await
    }

    /// Detach this clone's API-client cell. MUST be called after
    /// mutating `config.agent_instance_hierarchy` or
    /// `config.mcp_session_id` on a cloned `Context`: the memoized
    /// `HttpClient` snapshots those into headers, so a shared cell
    /// would either serve the base identity to this clone or poison
    /// the base context with this clone's identity. The viewer/db
    /// cells stay shared — neither client carries identity.
    pub fn reset_api_client(&mut self) {
        self.api = Arc::new(OnceCell::new());
    }
}

/// Build the SDK `HttpClient` for this cli process. `address` is the
/// already-resolved API base URL; every other field's precedence is
/// `env var → on-disk config (merged final view) → SDK default`. The
/// SDK's `env` feature is still enabled in `Cargo.toml`, but every
/// value we pass in is already resolved (Some/None) so the SDK never
/// reaches its own env fallback — the precedence chain is ours.
///
/// The viewer-mirror headers (`x_viewer_signature`,
/// `x_viewer_address`) are deliberately not set — viewer discovery is
/// lock-based now and the API client no longer carries them.
///
/// Sourcing `agent_instance_hierarchy` and `mcp_session_id` from
/// `cli_config` is deliberate: those are env-populated at startup by
/// `EnvConfigBuilder` *and* mutated for per-request overrides at the
/// MCP boundary (see the per-call `X-OBJECTIVEAI-AGENT-INSTANCE-
/// HIERARCHY` header stamp in `objectiveai-mcp`). Re-reading the env
/// here would silently drop those overrides.
fn build_http_client(
    cli_config: &Config,
    config: &mut crate::filesystem::config::Config,
    address: String,
) -> HttpClient {
    // Every header value comes from the FINAL merged config view — never
    // re-read from process env here. The config layer is the single source
    // of truth (env is already folded in at the appropriate precedence when
    // the view is built), so reading env again would double-source values.
    let authorization =
        config.api().get_objectiveai_authorization().map(String::from);

    let user_agent = config.api().get_user_agent().map(String::from);

    let x_title = config.api().get_x_title().map(String::from);

    let http_referer = config.api().get_http_referer().map(String::from);

    let x_github_authorization =
        config.api().get_github_authorization().map(String::from);

    let x_openrouter_authorization =
        config.api().get_openrouter_authorization().map(String::from);

    let x_mcp_authorization: Option<std::collections::HashMap<String, String>> =
        config
            .api()
            .get_mcp_authorization()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

    // From cli_config (env-populated at startup, mutable per request
    // at the MCP boundary) — never re-read from env here.
    let agent_instance_hierarchy = Some(cli_config.agent_instance_hierarchy.clone());
    let mcp_session_id = cli_config.mcp_session_id.clone();

    HttpClient::new(
        reqwest::Client::new(),
        Some(address),
        authorization,
        user_agent,
        x_title,
        http_referer,
        x_github_authorization,
        x_openrouter_authorization,
        x_mcp_authorization,
        None::<String>,
        None::<String>,
        agent_instance_hierarchy,
        mcp_session_id,
    )
}

/// Ensure an explicit scheme on a configured address: leave
/// `http://`/`https://` untouched, otherwise prefix `http://` —
/// regardless of whether the host is an IPv4, IPv6, or hostname.
pub(crate) fn ensure_scheme(a: &str) -> String {
    if a.starts_with("http://") || a.starts_with("https://") {
        a.to_string()
    } else {
        format!("http://{a}")
    }
}

/// Parse `(host:port, user, password)` out of the db spawn lock's
/// published URL. The format is OUR OWN — `objectiveai-db` publishes
/// exactly `postgresql://{user}:{percent-encoded password}@{host}:{port}`
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

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

#[cfg(test)]
mod tests {
    use super::ensure_scheme;

    #[test]
    fn ensure_scheme_cases() {
        assert_eq!(ensure_scheme("127.0.0.1"), "http://127.0.0.1");
        assert_eq!(ensure_scheme("127.0.0.1:8080"), "http://127.0.0.1:8080");
        assert_eq!(ensure_scheme("::1"), "http://::1");
        assert_eq!(ensure_scheme("api.x"), "http://api.x");
        assert_eq!(ensure_scheme("http://api.x"), "http://api.x");
        assert_eq!(ensure_scheme("https://api.x"), "https://api.x");
    }
}
