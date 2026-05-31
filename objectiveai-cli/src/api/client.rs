//! Builds the SDK `HttpClient` for endpoint subcommands.
//!
//! Precedence for every field is `env var → config file → SDK default`,
//! except `agent_id` which is sourced from the top-level `cli::Config`
//! (already env-populated by `EnvConfigBuilder` at startup) so per-
//! request overrides — e.g. the MCP server's per-call header stamp —
//! flow through to outbound HTTP. The cli disabled the SDK's `env`
//! feature, so this module re-implements the env-var lookup itself and
//! chains it with the on-disk `ApiConfig` / `ViewerConfig` before
//! handing the resolved values to `objectiveai_sdk::HttpClient::new`.

/// Compose `(addr, port)` into a base URL, ensuring an explicit scheme.
///
/// Rule: if the address already starts with `http://` or `https://`, leave
/// it untouched. Otherwise prefix `http://` — regardless of whether the
/// host is an IPv4, IPv6, or hostname.
pub(crate) fn compose_url(addr: Option<&str>, port: Option<u16>) -> Option<String> {
    fn ensure_scheme(a: &str) -> String {
        if a.starts_with("http://") || a.starts_with("https://") {
            a.to_string()
        } else {
            format!("http://{a}")
        }
    }
    match (addr, port) {
        (Some(a), Some(p)) => Some(format!("{}:{p}", ensure_scheme(a).trim_end_matches('/'))),
        (Some(a), None) => Some(ensure_scheme(a)),
        (None, Some(p)) => Some(format!("http://127.0.0.1:{p}")),
        (None, None) => None,
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

pub fn build_http_client(
    cli_config: &crate::Config,
    config: &mut objectiveai_sdk::filesystem::config::Config,
) -> objectiveai_sdk::HttpClient {
    let address = env("OBJECTIVEAI_ADDRESS").or_else(|| {
        let api = config.api();
        compose_url(api.get_address(), api.get_port())
    });

    let authorization = env("OBJECTIVEAI_AUTHORIZATION")
        .or_else(|| config.api().get_objectiveai_authorization().map(String::from));

    let user_agent = env("USER_AGENT")
        .or_else(|| config.api().get_user_agent().map(String::from));

    let x_title = env("X_TITLE")
        .or_else(|| config.api().get_x_title().map(String::from));

    let http_referer = env("HTTP_REFERER")
        .or_else(|| config.api().get_http_referer().map(String::from));

    let x_github_authorization = env("GITHUB_AUTHORIZATION")
        .or_else(|| config.api().get_github_authorization().map(String::from));

    let x_openrouter_authorization = env("OPENROUTER_AUTHORIZATION")
        .or_else(|| config.api().get_openrouter_authorization().map(String::from));

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

    // Source from the top-level cli::Config rather than re-reading the
    // env: `EnvConfigBuilder` already loads OBJECTIVEAI_AGENT_ID into
    // cli_config.agent_id at startup, and per-request overrides
    // (notably the MCP server's per-call X-OBJECTIVEAI-AGENT-ID stamp
    // in objectiveai-mcp/src/objectiveai.rs::run_cli_and_collect) mutate
    // this clone of cli_config — reading from env here would silently
    // drop those overrides and truncate the agent lineage at every
    // MCP-spawn boundary.
    let agent_id = Some(cli_config.agent_id.clone());
    let mcp_session_id = cli_config.mcp_session_id.clone();

    objectiveai_sdk::HttpClient::new(
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
        agent_id,
        mcp_session_id,
    )
}

/// Build the SDK's fire-and-forget viewer client with the same
/// env→ViewerConfig resolution `build_http_client` uses for the
/// `HttpClient`'s `x_viewer_*` fields (`VIEWER_ADDRESS` env →
/// `compose_url(viewer.address, viewer.port)`; `VIEWER_SIGNATURE` env →
/// `viewer.signature`).
///
/// Returned client owns an unbounded mpsc + a background tokio task
/// that POSTs each enqueued request with exponential-backoff retry.
/// `max_elapsed_time` is short (5s) because cli callers are
/// interactive — beyond that the user has moved on and the
/// notification is stale. Cli callers should `.flush().await` before
/// returning so the bg task drains before the runtime drops.
pub fn build_viewer_client(
    config: &mut objectiveai_sdk::filesystem::config::Config,
) -> objectiveai_sdk::http::viewer::Client {
    let address = env("VIEWER_ADDRESS").or_else(|| {
        let viewer = config.viewer();
        compose_url(viewer.get_address(), viewer.get_port())
    });
    let signature = env("VIEWER_SIGNATURE")
        .or_else(|| config.viewer().get_signature().map(String::from));

    objectiveai_sdk::http::viewer::Client::new(
        reqwest::Client::new(),
        address,
        signature,
        std::time::Duration::from_millis(100),
        std::time::Duration::from_millis(100),
        0.5,
        2.0,
        std::time::Duration::from_secs(2),
        std::time::Duration::from_secs(5),
    )
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
        assert_eq!(
            compose_url(Some("::1"), None).as_deref(),
            Some("http://::1"),
        );
        assert_eq!(
            compose_url(Some("api.x"), None).as_deref(),
            Some("http://api.x"),
        );
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
