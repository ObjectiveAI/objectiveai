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
    /// Lazily-connected db pool — see [`Context::db_client`]. No
    /// per-request identity; always shared across clones.
    db: Arc<OnceCell<db::Pool>>,
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
        }
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

    /// The db pool, connected on first use (ensuring the application
    /// database + schema exist) and memoized. Must NOT connect
    /// eagerly: commands like `db config ...` and `db spawn` have to
    /// work before any database exists — they're how you bring one up
    /// in the first place.
    ///
    /// URL resolution: when `db.address` is set in the merged config
    /// view, a remote-postgres URL is composed from the `config db`
    /// parts; otherwise the `db spawn` flow returns the local
    /// cluster's lock-published `postgresql://` URL (starting the
    /// objectiveai-db supervisor if needed).
    pub async fn db_client(&self) -> Result<&db::Pool, crate::error::Error> {
        self.db
            .get_or_try_init(|| async {
                let mut config = self
                    .filesystem
                    .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
                    .await?;
                let address = config.db().get_address().map(String::from);
                let url = match address {
                    Some(address) => {
                        let db = config.db();
                        db::config_url(
                            &address,
                            db.get_user()
                                .unwrap_or(crate::filesystem::config::DB_DEFAULT_USER),
                            db.get_password()
                                .unwrap_or(crate::filesystem::config::DB_DEFAULT_PASSWORD),
                        )
                    }
                    None => crate::command::db::spawn::spawn(self).await?,
                };
                let database = config
                    .db()
                    .get_database()
                    .unwrap_or(crate::filesystem::config::DB_DEFAULT_DATABASE)
                    .to_string();
                Ok(db::init(&url, &database).await?)
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
    let authorization = env("OBJECTIVEAI_AUTHORIZATION").or_else(|| {
        config
            .api()
            .get_objectiveai_authorization()
            .map(String::from)
    });

    let user_agent =
        env("USER_AGENT").or_else(|| config.api().get_user_agent().map(String::from));

    let x_title =
        env("X_TITLE").or_else(|| config.api().get_x_title().map(String::from));

    let http_referer = env("HTTP_REFERER")
        .or_else(|| config.api().get_http_referer().map(String::from));

    let x_github_authorization = env("GITHUB_AUTHORIZATION")
        .or_else(|| config.api().get_github_authorization().map(String::from));

    let x_openrouter_authorization = env("OPENROUTER_AUTHORIZATION").or_else(|| {
        config
            .api()
            .get_openrouter_authorization()
            .map(String::from)
    });

    let x_mcp_authorization: Option<std::collections::HashMap<String, String>> =
        env("MCP_AUTHORIZATION")
            .and_then(|v| serde_json::from_str(&v).ok())
            .or_else(|| {
                config
                    .api()
                    .get_mcp_authorization()
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            });

    let x_commit_author_name = env("COMMIT_AUTHOR_NAME")
        .or_else(|| config.api().get_commit_author_name().map(String::from));

    let x_commit_author_email = env("COMMIT_AUTHOR_EMAIL")
        .or_else(|| config.api().get_commit_author_email().map(String::from));

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
        x_commit_author_name,
        x_commit_author_email,
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
