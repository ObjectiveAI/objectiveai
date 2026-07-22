//! Per-request state — [`ScopedContext`].
//!
//! A scope is CONSTRUCTED, never mutated: identity is set once (the
//! `/execute` arrival is the main driver — see [`Self::for_request`])
//! and immutable thereafter, and the API-client cell is born fresh
//! alongside every new identity. The identity-poisoning hazard that
//! used to require `reset_api_client` is therefore unrepresentable —
//! there is no way to hold an `HttpClient` whose identity headers
//! disagree with the scope that owns it.
//!
//! The filesystem client lives here (not on the global) because it
//! carries config-derived state: [`Self::for_request`] re-resolves
//! `commit_author_name`/`email` from the merged on-disk config view at
//! construction, so `api config commit-author-name set` takes effect
//! on the next request with no daemon restart. Construction logic,
//! not mutation logic.

use std::sync::Arc;

use objectiveai_sdk::HttpClient;
use objectiveai_sdk::cli::command::AgentArguments;
use tokio::sync::OnceCell;

use crate::context::GlobalContext;
use crate::filesystem;
use crate::run::Config;

/// The identity fields as a plain value — the ONLY currency for
/// obtaining a differently-identified scope (via
/// [`ScopedContext::for_request`]). The three `plugin_*` fields are
/// the PLUGIN CALLER identity (which installed plugin originated the
/// work); absent for non-plugin callers.
pub struct ScopeIdentity {
    pub agent_instance_hierarchy: String,
    pub agent_id: Option<String>,
    pub agent_full_id: Option<String>,
    pub agent_remote: Option<String>,
    pub response_id: Option<String>,
    pub response_ids: Option<String>,
    pub plugin_owner: Option<String>,
    pub plugin_name: Option<String>,
    pub plugin_version: Option<String>,
    /// Fired by the task scheduler — daemon-authored like the plugin
    /// trio: never taken from a wire envelope, set only via
    /// [`ScopedContext::with_task`].
    pub task: bool,
}

impl ScopeIdentity {
    /// The daemon's scrubbed default identity
    /// (`agent_instance_hierarchy = "daemon"`, every other field
    /// cleared) — mirrors `ConfigBuilder::build`'s defaults. Used by
    /// detached daemon tasks, which must run exactly as the daemon's
    /// own env-scrubbed identity regardless of who triggered them.
    /// (A plain-user CLI invocation is `"cli"` — stamped per request
    /// by the `objectiveai` binary's envelope, never defaulted here.)
    pub fn default_daemon() -> Self {
        Self {
            agent_instance_hierarchy: "daemon".to_string(),
            agent_id: None,
            agent_full_id: None,
            agent_remote: None,
            response_id: None,
            response_ids: None,
            plugin_owner: None,
            plugin_name: None,
            plugin_version: None,
            task: false,
        }
    }

    /// The per-request identity from a `/execute` envelope. Every
    /// field is taken verbatim — a `None` DELETES the slot (never
    /// inherits the base scope's value) — and
    /// `agent_instance_hierarchy` falls back to `"UNKNOWN"` when
    /// missing because it is non-nullable.
    pub fn from_agent_arguments(args: &AgentArguments) -> Self {
        Self {
            agent_instance_hierarchy: args
                .agent_instance_hierarchy
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            agent_id: args.agent_id.clone(),
            agent_full_id: args.agent_full_id.clone(),
            agent_remote: args.agent_remote.clone(),
            response_id: args.response_id.clone(),
            response_ids: args.response_ids.clone(),
            // The plugin trio is NEVER taken from a wire envelope —
            // plugin caller identity is unspoofable; only the
            // daemon's own `plugins run` stamps it, via
            // [`ScopedContext::with_plugin`] on the nested scope.
            plugin_owner: None,
            plugin_name: None,
            plugin_version: None,
            // Daemon-authored like the trio — never taken from a wire
            // envelope; the task scheduler re-stamps via `with_task`.
            task: false,
        }
    }
}

#[derive(Clone)]
pub struct ScopedContext {
    /// Per-scope layout + config client. `dir`/`state` are the boot
    /// facts (they define WHERE config lives, so config cannot move
    /// them); the commit-author fields are re-resolved from the
    /// on-disk config at scope construction ([`Self::for_request`]).
    pub filesystem: filesystem::Client,
    // The identity fields: private and immutable — a different
    // identity means a different scope (`for_request`), never a
    // mutation. The `plugin_*` trio is the plugin caller identity,
    // stamped by `plugins run` (env or [`Self::with_plugin`]).
    agent_instance_hierarchy: String,
    agent_id: Option<String>,
    agent_full_id: Option<String>,
    agent_remote: Option<String>,
    response_id: Option<String>,
    response_ids: Option<String>,
    plugin_owner: Option<String>,
    plugin_name: Option<String>,
    plugin_version: Option<String>,
    /// Task-scheduler-fired run marker — daemon-authored (see
    /// [`ScopeIdentity::task`]); set only by [`Self::with_task`].
    task: bool,
    /// When true, the embedded python's `objectiveai.execute(...)`
    /// host call raises instead of dispatching a CLI command. Set by
    /// the `python --no-objectiveai` flag and automatically for the
    /// per-stream-item python output transform. Copy field, carried
    /// per-clone — see [`Self::with_no_objectiveai`].
    pub no_objectiveai: bool,
    /// Lazily-built API `HttpClient` — see [`Self::api_client`]. Born
    /// fresh with the scope (a new identity can never observe a stale
    /// client); shared only among clones of the SAME scope, whose
    /// identity is identical by construction.
    api: Arc<OnceCell<HttpClient>>,
}

impl ScopedContext {
    /// The boot scope: sync and IO-free, from the env-parsed [`Config`]
    /// (filesystem from its dir/state/authors, identity from its seven
    /// env-populated fields). Used by `run`'s no-context arm and
    /// tests; per-request scopes derive from it via
    /// [`Self::for_request`].
    pub fn boot(config: &Config) -> Self {
        let filesystem = filesystem::Client::new(
            config.objectiveai_dir.clone(),
            config.objectiveai_state.clone(),
            config.commit_author_name.clone(),
            config.commit_author_email.clone(),
        );
        Self {
            filesystem,
            agent_instance_hierarchy: config.agent_instance_hierarchy.clone(),
            agent_id: config.agent_id.clone(),
            agent_full_id: config.agent_full_id.clone(),
            agent_remote: config.agent_remote.clone(),
            response_id: config.response_id.clone(),
            response_ids: config.response_ids.clone(),
            // The daemon's own boot identity is never a plugin's; the
            // trio arrives per request (envelope headers) or via
            // `with_plugin` (nested plugin commands).
            plugin_owner: None,
            plugin_name: None,
            plugin_version: None,
            task: false,
            no_objectiveai: false,
            api: Arc::new(OnceCell::new()),
        }
    }

    /// THE per-request constructor: a NEW scope with the given
    /// identity, a FRESH api cell, and a filesystem rebuilt with the
    /// commit author resolved from the CURRENT merged config view
    /// (config → this scope's values as fallback, also on a read
    /// error — best-effort). `dir`/`state` carry over unchanged: they
    /// pin the daemon itself. Async because construction reads
    /// config; nothing is ever mutated.
    pub async fn for_request(&self, identity: ScopeIdentity) -> Self {
        let (commit_author_name, commit_author_email) = match self
            .filesystem
            .read_config()
            .await
        {
            Ok(mut config) => {
                let api = config.api();
                (
                    api.get_commit_author_name()
                        .map(String::from)
                        .unwrap_or_else(|| self.filesystem.commit_author_name.clone()),
                    api.get_commit_author_email()
                        .map(String::from)
                        .unwrap_or_else(|| self.filesystem.commit_author_email.clone()),
                )
            }
            Err(_) => (
                self.filesystem.commit_author_name.clone(),
                self.filesystem.commit_author_email.clone(),
            ),
        };
        let filesystem = filesystem::Client::new(
            Some(self.filesystem.dir().clone()),
            Some(self.filesystem.state().to_string()),
            Some(commit_author_name),
            Some(commit_author_email),
        );
        Self {
            filesystem,
            agent_instance_hierarchy: identity.agent_instance_hierarchy,
            agent_id: identity.agent_id,
            agent_full_id: identity.agent_full_id,
            agent_remote: identity.agent_remote,
            response_id: identity.response_id,
            response_ids: identity.response_ids,
            plugin_owner: identity.plugin_owner,
            plugin_name: identity.plugin_name,
            plugin_version: identity.plugin_version,
            task: identity.task,
            no_objectiveai: self.no_objectiveai,
            api: Arc::new(OnceCell::new()),
        }
    }

    /// Derive a clone with `objectiveai.execute` inside the embedded
    /// python gated to raise instead of dispatch. The api cell stays
    /// SHARED — the flag never touches identity headers.
    pub fn with_no_objectiveai(&self, no_objectiveai: bool) -> Self {
        let mut clone = self.clone();
        clone.no_objectiveai = no_objectiveai;
        clone
    }

    pub fn agent_instance_hierarchy(&self) -> &str {
        &self.agent_instance_hierarchy
    }

    pub fn agent_id(&self) -> Option<&str> {
        self.agent_id.as_deref()
    }

    pub fn agent_full_id(&self) -> Option<&str> {
        self.agent_full_id.as_deref()
    }

    pub fn agent_remote(&self) -> Option<&str> {
        self.agent_remote.as_deref()
    }

    pub fn response_id(&self) -> Option<&str> {
        self.response_id.as_deref()
    }

    pub fn response_ids(&self) -> Option<&str> {
        self.response_ids.as_deref()
    }

    pub fn plugin_owner(&self) -> Option<&str> {
        self.plugin_owner.as_deref()
    }

    pub fn plugin_name(&self) -> Option<&str> {
        self.plugin_name.as_deref()
    }

    pub fn plugin_version(&self) -> Option<&str> {
        self.plugin_version.as_deref()
    }

    /// Derive a clone stamped with the PLUGIN CALLER identity — used
    /// by `plugins run` so every nested (plugin-originated) command's
    /// scope carries which installed plugin it came from. Like
    /// [`Self::with_no_objectiveai`], the api cell stays shared: the
    /// plugin trio never rides identity headers to the API.
    pub fn with_plugin(
        &self,
        owner: impl Into<String>,
        repository: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        let mut clone = self.clone();
        clone.plugin_owner = Some(owner.into());
        clone.plugin_name = Some(repository.into());
        clone.plugin_version = Some(version.into());
        clone
    }

    /// Whether this scope is a task-scheduler-fired run.
    pub fn task(&self) -> bool {
        self.task
    }

    /// Derive a clone marked as a task-scheduler-fired run — the
    /// SCHEDULER is the only caller (daemon-authored, like
    /// [`Self::with_plugin`] for the trio).
    pub fn with_task(&self) -> Self {
        let mut clone = self.clone();
        clone.task = true;
        clone
    }

    /// The API `HttpClient`, built on first use and memoized for this
    /// scope's life. Its identity headers come from THIS scope, so the
    /// cell can never serve another identity — that invariant is the
    /// whole reason the cell lives here and not on the global.
    ///
    /// Address resolution: `api.address` from the merged (`--final`)
    /// config view when set, else the internal api spawn flow — which
    /// returns the announced URL of an already-running api without
    /// spawning, or starts one and waits for its ready line.
    pub async fn api_client(
        &self,
        global: &GlobalContext,
    ) -> Result<&HttpClient, crate::error::Error> {
        self.api
            .get_or_try_init(|| async {
                let mut config = self
                    .filesystem
                    .read_config()
                    .await?;
                let address = match config.api().get_address() {
                    Some(a) => ensure_scheme(a),
                    None => crate::command::api::spawn::spawn(global).await?,
                };
                Ok(build_http_client(global.http().clone(), self, &mut config, address))
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
        global: &GlobalContext,
    ) -> Result<HttpClient, crate::error::Error> {
        let mut config = self
            .filesystem
            .read_config()
            .await?;
        // Tripwire address — this client must only be used for github_* calls.
        let address = "https://api.invalid.objectiveai-github-client-only/".to_string();
        Ok(build_http_client(global.http().clone(), self, &mut config, address))
    }
}

/// Build the SDK `HttpClient` for one scope. `address` is the
/// already-resolved API base URL; `http` is the process-wide `reqwest`
/// pool off the [`GlobalContext`] (scopes differ only in headers, so
/// they share the TCP/TLS pool). Every other field's precedence is
/// `env var → on-disk config (merged final view) → SDK default`, all
/// already folded into the passed config view.
///
/// Sourcing `agent_instance_hierarchy` from the SCOPE is deliberate:
/// it is per-request identity (env-populated at boot, overridden per
/// `/execute` envelope). Re-reading the env here would silently drop
/// the overrides.
fn build_http_client(
    http: reqwest::Client,
    scoped: &ScopedContext,
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

    // From the scope — the per-request identity, never re-read from env.
    let agent_instance_hierarchy = Some(scoped.agent_instance_hierarchy().to_string());

    // The per-request MCP CALL budget the API applies on our behalf,
    // sent as `X-MCP-CALL-TIMEOUT`. Sourced from
    // `api.mcp_call_timeout_ms`; unset ⇒ no header ⇒ the API applies NO
    // call timeout.
    let mcp_call_timeout_ms = config.api().get_mcp_call_timeout_ms();

    HttpClient::new(
        http,
        Some(address),
        authorization,
        user_agent,
        x_title,
        http_referer,
        x_github_authorization,
        x_openrouter_authorization,
        x_mcp_authorization,
        agent_instance_hierarchy,
        mcp_call_timeout_ms,
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
