use std::future::Future;
use std::sync::Arc;
use envconfig::Envconfig;

/// Reads config and dispatches to the appropriate run variant
/// based on API mode (Local/Remote) and Viewer mode (Local/Remote).
///
/// If `viewer` is false, the viewer mode is forced to Remote regardless of
/// config — no point spawning a viewer window if nothing will be displayed.
pub async fn run<F, Fut>(task: F, viewer: bool) -> Result<crate::Output, crate::error::Error>
where
    F: FnOnce(objectiveai::HttpClient) -> Fut + Send + 'static,
    Fut: Future<Output = Result<String, crate::error::Error>> + Send + 'static,
{
    let client = objectiveai::filesystem::Client::new(None::<String>, None::<String>, None::<String>);
    // Note: api::run creates its own Client without cli_config context.
    // Commit author fields fall back to env vars or defaults.
    let mut config = objectiveai::filesystem::config::client::read(&client).await?;

    let api_mode = config.api().get_mode();
    let viewer_mode = if viewer {
        config.viewer().get_mode()
    } else {
        objectiveai::filesystem::config::ViewerMode::Remote
    };

    match (api_mode, viewer_mode) {
        #[cfg(feature = "viewer")]
        (objectiveai::filesystem::config::ApiMode::Local, objectiveai::filesystem::config::ViewerMode::Local) => {
            run_local_api_local_viewer(config, task).await
        }
        #[cfg(not(feature = "viewer"))]
        (objectiveai::filesystem::config::ApiMode::Local, objectiveai::filesystem::config::ViewerMode::Local) => {
            run_local_api_remote_viewer(config, task).await
        }
        (objectiveai::filesystem::config::ApiMode::Local, objectiveai::filesystem::config::ViewerMode::Remote) => {
            run_local_api_remote_viewer(config, task).await
        }
        #[cfg(feature = "viewer")]
        (objectiveai::filesystem::config::ApiMode::Remote, objectiveai::filesystem::config::ViewerMode::Local) => {
            run_remote_api_local_viewer(config, task).await
        }
        #[cfg(not(feature = "viewer"))]
        (objectiveai::filesystem::config::ApiMode::Remote, objectiveai::filesystem::config::ViewerMode::Local) => {
            run_remote_api_remote_viewer(config, task).await
        }
        (objectiveai::filesystem::config::ApiMode::Remote, objectiveai::filesystem::config::ViewerMode::Remote) => {
            run_remote_api_remote_viewer(config, task).await
        }
    }
    .map(crate::Output::Api)
}

// -- Variants --

/// Spawns both a local API server and a local viewer subprocess.
/// The API's viewer client is pointed at the viewer's bound address.
/// The viewer subprocess is killed when the task completes.
#[cfg(feature = "viewer")]
async fn run_local_api_local_viewer<F, Fut>(
    mut config: objectiveai::filesystem::config::Config,
    task: F,
) -> Result<String, crate::error::Error>
where
    F: FnOnce(objectiveai::HttpClient) -> Fut + Send + 'static,
    Fut: Future<Output = Result<String, crate::error::Error>> + Send + 'static,
{
    let (secret, secret_from_env, config_signature) = resolve_viewer_secret(&mut config)?;

    // ENV mismatch check: VIEWER_SECRET and VIEWER_SIGNATURE must both or neither come from ENV
    let api_builder_peek = objectiveai_api::ConfigBuilder::init_from_env().unwrap_or_default();
    if secret_from_env != api_builder_peek.viewer_signature.is_some() {
        return Err(crate::error::Error::ViewerSecretSignatureEnvMismatch);
    }

    // Spawn viewer subprocess and wait for it to report its bound address
    let (mut viewer_child, viewer_addr_str) = spawn_viewer(secret.as_deref(), &mut config).await?;

    // Setup API with viewer address + signature so its viewer client can POST events
    let api_config = build_api_config(&mut config, Some(viewer_addr_str.clone()), config_signature.clone());

    let (api_listener, api_router) = objectiveai_api::setup(api_config).await
        .map_err(crate::error::Error::ApiSetup)?;
    let api_addr = api_listener.local_addr()
        .map_err(crate::error::Error::ApiSetup)?;
    tokio::spawn(async move {
        let _ = objectiveai_api::serve(api_listener, api_router).await;
    });

    // HttpClient points at the local API, with viewer headers for event forwarding
    let http_client = build_http_client(
        &mut config,
        Some(format!("http://127.0.0.1:{}", api_addr.port())),
        Some(viewer_addr_str),
        config_signature,
    );

    let result = task(http_client).await;
    let _ = viewer_child.kill().await;
    result
}

/// Spawns a local API server only. No viewer window — viewer address and
/// signature come from ENV/config headers (pointing at a remote viewer).
async fn run_local_api_remote_viewer<F, Fut>(
    mut config: objectiveai::filesystem::config::Config,
    task: F,
) -> Result<String, crate::error::Error>
where
    F: FnOnce(objectiveai::HttpClient) -> Fut + Send + 'static,
    Fut: Future<Output = Result<String, crate::error::Error>> + Send + 'static,
{
    // Viewer fields overlay from headers (remote viewer configured externally)
    let api_config = build_api_config(&mut config, None, None);

    let (api_listener, api_router) = objectiveai_api::setup(api_config).await
        .map_err(crate::error::Error::ApiSetup)?;
    let api_addr = api_listener.local_addr()
        .map_err(crate::error::Error::ApiSetup)?;
    tokio::spawn(async move {
        let _ = objectiveai_api::serve(api_listener, api_router).await;
    });

    // HttpClient points at the local API; viewer fields from ENV/config
    let http_client = build_http_client(
        &mut config,
        Some(format!("http://127.0.0.1:{}", api_addr.port())),
        None,
        None,
    );

    task(http_client).await
}

/// Spawns a local viewer subprocess only. The API is remote — the task's
/// HttpClient sends viewer headers so the remote API can forward events
/// to our local viewer.
#[cfg(feature = "viewer")]
async fn run_remote_api_local_viewer<F, Fut>(
    mut config: objectiveai::filesystem::config::Config,
    task: F,
) -> Result<String, crate::error::Error>
where
    F: FnOnce(objectiveai::HttpClient) -> Fut + Send + 'static,
    Fut: Future<Output = Result<String, crate::error::Error>> + Send + 'static,
{
    let (secret, _, config_signature) = resolve_viewer_secret(&mut config)?;

    // Spawn viewer subprocess and wait for it to report its bound address
    let (mut viewer_child, viewer_addr_str) = spawn_viewer(secret.as_deref(), &mut config).await?;

    // HttpClient points at the remote API, with local viewer address + signature
    let http_client = build_http_client(
        &mut config,
        None,
        Some(viewer_addr_str),
        config_signature,
    );

    let result = task(http_client).await;
    let _ = viewer_child.kill().await;
    result
}

/// No local spawning. The API and viewer are both remote.
/// HttpClient gets all values from ENV and config file.
async fn run_remote_api_remote_viewer<F, Fut>(
    mut config: objectiveai::filesystem::config::Config,
    task: F,
) -> Result<String, crate::error::Error>
where
    F: FnOnce(objectiveai::HttpClient) -> Fut + Send + 'static,
    Fut: Future<Output = Result<String, crate::error::Error>> + Send + 'static,
{
    let http_client = build_http_client(&mut config, None, None, None);
    task(http_client).await
}

// -- Viewer subprocess --

/// Pre-built viewer binary, embedded at compile time by build.rs.
#[cfg(feature = "viewer")]
const VIEWER_BINARY: &[u8] = include_bytes!(env!("OBJECTIVEAI_VIEWER_BINARY_PATH"));

/// Extracts the embedded viewer binary to a hash-keyed temp directory and returns its path.
///
/// Uses a content-based hash in the directory name so different CLI versions get separate
/// binaries and the same version reuses the cached binary across restarts.
#[cfg(feature = "viewer")]
fn extract_viewer_binary() -> Result<std::path::PathBuf, crate::error::Error> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    VIEWER_BINARY.len().hash(&mut hasher);
    VIEWER_BINARY[..VIEWER_BINARY.len().min(4096)].hash(&mut hasher);
    VIEWER_BINARY[VIEWER_BINARY.len().saturating_sub(4096)..].hash(&mut hasher);
    let hash = hasher.finish();

    let name = if cfg!(windows) { "objectiveai-viewer.exe" } else { "objectiveai-viewer" };
    let dir = std::env::temp_dir().join(format!("objectiveai-viewer-{hash:016x}"));
    let path = dir.join(name);

    if !path.exists() {
        std::fs::create_dir_all(&dir).map_err(crate::error::Error::ViewerSpawn)?;
        std::fs::write(&path, VIEWER_BINARY).map_err(crate::error::Error::ViewerSpawn)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .map_err(crate::error::Error::ViewerSpawn)?;
        }
    }
    Ok(path)
}

/// Spawns the viewer as a subprocess and reads its bound address from stderr.
///
/// The viewer prints `listening on <addr>` to stderr after binding its HTTP server.
/// Returns the child process handle and the viewer's HTTP address.
///
/// The viewer ends up with the same effective HttpClient configuration as the
/// in-process API server: parent ENV passes through automatically, and any
/// fields backed only by the config-file headers are forwarded explicitly via
/// `cmd.env(...)`.
#[cfg(feature = "viewer")]
async fn spawn_viewer(
    secret: Option<&str>,
    config: &mut objectiveai::filesystem::config::Config,
) -> Result<(tokio::process::Child, String), crate::error::Error> {
    let viewer_path = extract_viewer_binary()?;

    let mut cmd = tokio::process::Command::new(&viewer_path);
    cmd.env("ADDRESS", "127.0.0.1");
    cmd.env("PORT", "0");
    if let Some(s) = secret {
        cmd.env("VIEWER_SECRET", s);
    }

    // Forward HttpClient config from the config-file headers when ENV doesn't
    // already provide it. ENV vars set in the parent process inherit into the
    // child automatically, so we only need to set those backed solely by the
    // config file. This mirrors `build_api_config`'s overlay logic.
    let headers = config.api().headers();
    apply_env_overlay(&mut cmd, "OBJECTIVEAI_AUTHORIZATION", headers.get_x_objectiveai_authorization().map(String::from));
    apply_env_overlay(&mut cmd, "OPENROUTER_AUTHORIZATION", headers.get_x_openrouter_authorization().map(String::from));
    apply_env_overlay(&mut cmd, "GITHUB_AUTHORIZATION", headers.get_x_github_authorization().map(String::from));
    apply_env_overlay(&mut cmd, "MCP_AUTHORIZATION", headers.get_x_mcp_authorization().and_then(|m| serde_json::to_string(m).ok()));
    apply_env_overlay(&mut cmd, "USER_AGENT", headers.get_user_agent().map(String::from));
    apply_env_overlay(&mut cmd, "HTTP_REFERER", headers.get_http_referer().map(String::from));
    apply_env_overlay(&mut cmd, "X_TITLE", headers.get_x_title().map(String::from));
    apply_env_overlay(&mut cmd, "COMMIT_AUTHOR_NAME", headers.get_x_commit_author_name().map(String::from));
    apply_env_overlay(&mut cmd, "COMMIT_AUTHOR_EMAIL", headers.get_x_commit_author_email().map(String::from));
    apply_env_overlay(&mut cmd, "VIEWER_SIGNATURE", headers.get_x_viewer_signature().map(String::from));

    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(crate::error::Error::ViewerSpawn)?;

    let stderr = child.stderr.take().unwrap();
    let mut reader = tokio::io::BufReader::new(stderr);

    let mut line = String::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line),
    )
    .await
    .map_err(|_| crate::error::Error::ViewerProtocol)?
    .map_err(crate::error::Error::ViewerSpawn)?;

    // Parse "listening on <addr>" from the viewer's stderr
    let addr = line.trim().strip_prefix("listening on ").ok_or(crate::error::Error::ViewerProtocol)?;
    let viewer_addr = format!("http://{addr}");

    Ok((child, viewer_addr))
}

/// Sets `cmd.env(name, value)` only when `name` is unset in the parent
/// process and `fallback` is `Some`. Lets parent ENV win (it inherits to the
/// child anyway) while still propagating config-file-only values.
#[cfg(feature = "viewer")]
fn apply_env_overlay(cmd: &mut tokio::process::Command, name: &str, fallback: Option<String>) {
    if std::env::var_os(name).is_none() {
        if let Some(v) = fallback {
            cmd.env(name, v);
        }
    }
}

/// Resolves the viewer secret/signature pair from config.
/// Both must be present, or both absent. Returns `(secret, secret_from_env, signature)`.
#[cfg(feature = "viewer")]
fn resolve_viewer_secret(
    config: &mut objectiveai::filesystem::config::Config,
) -> Result<(Option<String>, bool, Option<String>), crate::error::Error> {
    let viewer_local = config.viewer().local();
    let (config_secret, config_signature) = match (viewer_local.get_secret(), viewer_local.get_signature()) {
        (Some(s), Some(sig)) => (Some(String::from(s)), Some(String::from(sig))),
        (None, None) => (None, None),
        _ => return Err(crate::error::Error::ViewerSecretSignatureConfigMismatch),
    };

    // Check if ENV provides a secret (takes priority over config)
    let env_secret = std::env::var("VIEWER_SECRET").ok();
    let secret_from_env = env_secret.is_some();
    let secret = env_secret.or(config_secret);

    Ok((secret, secret_from_env, config_signature))
}

// -- Shared helpers --

/// Builds the local API server config. Priority: ENV → config file → defaults.
///
/// Overlays authorization headers and `claude_agent_sdk_enabled` from the config file
/// for any fields not already set by ENV.
///
/// `viewer_address` and `viewer_signature` control how viewer fields are resolved:
/// - `Some(value)`: use this value (for local viewer — address is the bound port,
///   signature is from the config pair). Signature only sets if ENV didn't provide one.
/// - `None`: overlay from `ApiHeadersConfig` like all other fields (for remote viewer).
///
/// Always force-overrides `address`, `port`, and `suppress_output` for local binding.
fn build_api_config(
    config: &mut objectiveai::filesystem::config::Config,
    viewer_address: Option<String>,
    viewer_signature: Option<String>,
) -> objectiveai_api::Config {
    let mut builder = objectiveai_api::ConfigBuilder::init_from_env().unwrap_or_default();

    // Config file overlay: read claude_agent_sdk_enabled before borrowing headers
    builder.claude_agent_sdk_enabled = builder.claude_agent_sdk_enabled.or(config.api().local().get_claude_agent_sdk());

    // Config file overlay: fill None fields from ApiHeadersConfig
    let headers = config.api().headers();
    builder.objectiveai_authorization = builder.objectiveai_authorization.or(headers.get_x_objectiveai_authorization().map(String::from));
    builder.openrouter_authorization = builder.openrouter_authorization.or(headers.get_x_openrouter_authorization().map(String::from));
    builder.github_authorization = builder.github_authorization.or(headers.get_x_github_authorization().map(String::from));
    if builder.mcp_authorization.is_none() {
        builder.mcp_authorization = headers.get_x_mcp_authorization()
            .and_then(|m| serde_json::to_string(m).ok());
    }
    builder.user_agent = builder.user_agent.or(headers.get_user_agent().map(String::from));
    builder.http_referer = builder.http_referer.or(headers.get_http_referer().map(String::from));
    builder.x_title = builder.x_title.or(headers.get_x_title().map(String::from));
    builder.commit_author_name = builder.commit_author_name.or(headers.get_x_commit_author_name().map(String::from));
    builder.commit_author_email = builder.commit_author_email.or(headers.get_x_commit_author_email().map(String::from));

    // Viewer fields: override for local viewer, overlay from headers for remote
    match viewer_address {
        Some(addr) => builder.viewer_address = Some(addr),
        None => builder.viewer_address = builder.viewer_address.or(headers.get_x_viewer_address().map(String::from)),
    }
    match viewer_signature {
        Some(sig) => {
            if builder.viewer_signature.is_none() {
                builder.viewer_signature = Some(sig);
            }
        }
        None => builder.viewer_signature = builder.viewer_signature.or(headers.get_x_viewer_signature().map(String::from)),
    }

    // Force overrides: local API always binds to localhost on a system-assigned port
    builder.address = Some("127.0.0.1".to_string());
    builder.port = Some(0);
    builder.suppress_output = Some(true);

    builder.build()
}

/// Builds the SDK HttpClient for the task closure. Priority: ENV → config file → defaults.
///
/// `HttpClient::new` with all-None params picks up ENV vars via the `env` feature.
/// Then config file headers fill any remaining None fields.
///
/// - `address`: `Some` for local API (bound address), `None` for remote (env/default).
/// - `viewer_address`: `Some` for local viewer (bound address), `None` for remote (normal overlay).
/// - `viewer_signature`: `Some` for local viewer (config pair signature), `None` for remote.
///   Only sets if ENV and config overlay didn't already provide one.
fn build_http_client(
    config: &mut objectiveai::filesystem::config::Config,
    address: Option<String>,
    viewer_address: Option<String>,
    viewer_signature: Option<String>,
) -> objectiveai::HttpClient {
    // ENV fallbacks handled internally by HttpClient::new (env feature).
    // Viewer signature and address are passed directly so ENV takes priority
    // over the caller's values (HttpClient::new checks env first for None params,
    // but our params are Some when the caller provides overrides).
    let mut http_client = objectiveai::HttpClient::new(
        reqwest::Client::new(),
        address,          // local API bound addr, or None for remote (env/default)
        None::<String>,   // authorization
        None::<String>,   // user_agent
        None::<String>,   // x_title
        None::<String>,   // http_referer
        None::<String>,   // x_github_authorization
        None::<String>,   // x_openrouter_authorization
        None,             // x_mcp_authorization
        viewer_signature, // x_viewer_signature: local viewer pair, or None for remote
        viewer_address,   // x_viewer_address: local viewer bound addr, or None for remote
        None::<String>,   // x_commit_author_name
        None::<String>,   // x_commit_author_email
    );

    // Config file overlay: fill remaining None fields from ApiHeadersConfig
    let headers = config.api().headers();
    if http_client.authorization.is_none() {
        http_client.authorization = headers.get_x_objectiveai_authorization().map(|s| Arc::new(s.to_string()));
    }
    if http_client.user_agent.is_none() {
        http_client.user_agent = headers.get_user_agent().map(String::from);
    }
    if http_client.x_title.is_none() {
        http_client.x_title = headers.get_x_title().map(String::from);
    }
    if http_client.http_referer.is_none() {
        http_client.http_referer = headers.get_http_referer().map(String::from);
    }
    if http_client.x_github_authorization.is_none() {
        http_client.x_github_authorization = headers.get_x_github_authorization().map(|s| Arc::new(s.to_string()));
    }
    if http_client.x_openrouter_authorization.is_none() {
        http_client.x_openrouter_authorization = headers.get_x_openrouter_authorization().map(|s| Arc::new(s.to_string()));
    }
    if http_client.x_mcp_authorization.is_none() {
        http_client.x_mcp_authorization = headers.get_x_mcp_authorization()
            .map(|m| Arc::new(m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()));
    }
    if http_client.x_viewer_signature.is_none() {
        http_client.x_viewer_signature = headers.get_x_viewer_signature().map(|s| Arc::new(s.to_string()));
    }
    if http_client.x_viewer_address.is_none() {
        http_client.x_viewer_address = headers.get_x_viewer_address().map(|s| Arc::new(s.to_string()));
    }
    if http_client.x_commit_author_name.is_none() {
        http_client.x_commit_author_name = headers.get_x_commit_author_name().map(|s| Arc::new(s.to_string()));
    }
    if http_client.x_commit_author_email.is_none() {
        http_client.x_commit_author_email = headers.get_x_commit_author_email().map(|s| Arc::new(s.to_string()));
    }

    http_client
}
