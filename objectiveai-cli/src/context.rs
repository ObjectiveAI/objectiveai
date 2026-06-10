//! Process-wide state threaded into every bare-naked `command::*::execute`.
//!
//! `Context` is constructed once in `main.rs` (or by a programmatic
//! embedder), then borrowed into the command tree. Bodies that need IO
//! grab a `filesystem::Client` or the prebuilt `HttpClient` off it
//! rather than building one per call.
//!
//! The `HttpClient` builder mirrors the legacy `crate::api::client::
//! build_http_client` shape: per-field precedence is `env var →
//! on-disk config → SDK default`, except `agent_instance_hierarchy`
//! and `mcp_session_id` which come straight from `cli_config` (already
//! env-populated at startup, and mutated for per-request overrides at
//! the MCP boundary).

use objectiveai_sdk::HttpClient;

use crate::db;
use crate::filesystem;
use crate::plugin_path::PluginPath;
use crate::run::Config;

#[derive(Clone)]
pub struct Context {
    pub config: Config,
    pub filesystem: filesystem::Client,
    pub db: db::Pool,
    pub http: HttpClient,
    /// The plugin a command is running on behalf of, when it was
    /// invoked through a plugin's nested-command protocol. Assembled
    /// from the `OBJECTIVEAI_PLUGIN_*` env vars (via `Config`).
    pub plugin: Option<PluginPath>,
}

impl Context {
    pub async fn new(config: Config) -> Result<Self, crate::error::Error> {
        let filesystem = filesystem::Client::new(
            config.config_base_dir.clone(),
            config.commit_author_name.clone(),
            config.commit_author_email.clone(),
        );
        // Ensure the embedded postmaster is alive before opening the
        // application pool. `bootstrap` is fast on the warm path
        // (`postmaster.pid` already exists) and only spawns on first
        // run.
        crate::postgres::bootstrap(filesystem.base_dir()).await?;
        let db = db::init(filesystem.base_dir()).await?;
        let http = build_http_client(&config, &filesystem).await?;
        let plugin = PluginPath::from_parts(
            config.plugin_owner.clone(),
            config.plugin_repository.clone(),
            config.plugin_version.clone(),
        );
        Ok(Self {
            config,
            filesystem,
            db,
            http,
            plugin,
        })
    }
}

/// Build the SDK `HttpClient` for this cli process. Precedence per
/// field: `env var → on-disk config → SDK default`. The SDK's
/// `env` feature is still enabled in `Cargo.toml`, but every value
/// we pass in is already resolved (Some/None) so the SDK never
/// reaches its own env fallback — the precedence chain is ours.
///
/// Sourcing `agent_instance_hierarchy` and `mcp_session_id` from
/// `cli_config` is deliberate: those are env-populated at startup by
/// `EnvConfigBuilder` *and* mutated for per-request overrides at the
/// MCP boundary (see the per-call `X-OBJECTIVEAI-AGENT-INSTANCE-
/// HIERARCHY` header stamp in `objectiveai-mcp`). Re-reading the env
/// here would silently drop those overrides.
async fn build_http_client(
    cli_config: &Config,
    fs: &filesystem::Client,
) -> Result<HttpClient, crate::error::Error> {
    // The legacy filesystem `read_config` returns the full Config
    // struct. We mutate it through `&mut config.api()` etc., then
    // drop it — the resolved Option<String>s land in HttpClient by
    // value.
    let mut config = fs.read_config().await?;

    let address = env("OBJECTIVEAI_ADDRESS").or_else(|| {
        let api = config.api();
        compose_url(api.get_address(), api.get_port())
    });

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

    let x_viewer_signature = env("VIEWER_SIGNATURE")
        .or_else(|| config.viewer().get_signature().map(String::from));

    let x_viewer_address = env("VIEWER_ADDRESS").or_else(|| {
        let viewer = config.viewer();
        compose_url(viewer.get_address(), viewer.get_port())
    });

    let x_commit_author_name = env("COMMIT_AUTHOR_NAME")
        .or_else(|| config.api().get_commit_author_name().map(String::from));

    let x_commit_author_email = env("COMMIT_AUTHOR_EMAIL")
        .or_else(|| config.api().get_commit_author_email().map(String::from));

    // From cli_config (env-populated at startup, mutable per request
    // at the MCP boundary) — never re-read from env here.
    let agent_instance_hierarchy = Some(cli_config.agent_instance_hierarchy.clone());
    let mcp_session_id = cli_config.mcp_session_id.clone();

    Ok(HttpClient::new(
        reqwest::Client::new(),
        address,
        authorization,
        user_agent,
        x_title,
        http_referer,
        x_github_authorization,
        x_openrouter_authorization,
        x_mcp_authorization,
        x_viewer_signature,
        x_viewer_address,
        x_commit_author_name,
        x_commit_author_email,
        agent_instance_hierarchy,
        mcp_session_id,
    ))
}

/// Compose `(addr, port)` into a base URL, ensuring an explicit scheme.
///
/// Rule: if the address already starts with `http://` or `https://`,
/// leave it untouched. Otherwise prefix `http://` — regardless of
/// whether the host is an IPv4, IPv6, or hostname.
pub(crate) fn compose_url(addr: Option<&str>, port: Option<u16>) -> Option<String> {
    fn ensure_scheme(a: &str) -> String {
        if a.starts_with("http://") || a.starts_with("https://") {
            a.to_string()
        } else {
            format!("http://{a}")
        }
    }
    match (addr, port) {
        (Some(a), Some(p)) => {
            Some(format!("{}:{p}", ensure_scheme(a).trim_end_matches('/')))
        }
        (Some(a), None) => Some(ensure_scheme(a)),
        (None, Some(p)) => Some(format!("http://127.0.0.1:{p}")),
        (None, None) => None,
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

#[cfg(test)]
mod tests {
    use super::compose_url;

    #[test]
    fn compose_url_cases() {
        assert_eq!(compose_url(None, None), None);
        assert_eq!(
            compose_url(Some("127.0.0.1"), Some(8080)).as_deref(),
            Some("http://127.0.0.1:8080"),
        );
        assert_eq!(
            compose_url(Some("::1"), Some(8080)).as_deref(),
            Some("http://::1:8080"),
        );
        assert_eq!(compose_url(Some("::1"), None).as_deref(), Some("http://::1"));
        assert_eq!(compose_url(Some("api.x"), None).as_deref(), Some("http://api.x"));
        assert_eq!(
            compose_url(Some("https://api.x"), None).as_deref(),
            Some("https://api.x"),
        );
        assert_eq!(
            compose_url(Some("https://api.x"), Some(443)).as_deref(),
            Some("https://api.x:443"),
        );
        assert_eq!(
            compose_url(Some("https://api.x/"), Some(443)).as_deref(),
            Some("https://api.x:443"),
        );
        assert_eq!(
            compose_url(None, Some(8080)).as_deref(),
            Some("http://127.0.0.1:8080"),
        );
    }
}
