//! Viewer lifecycle: env config, the event bus, and the Tauri shell.
//!
//! The Rust side holds NO daemon stream: the JS frontend connects to
//! the daemon's published `ws://` endpoint directly (native
//! WebSockets to `/listen` and `/execute`), and the Rust side only
//! hands it the variables it needs via the [`websocket_config`]
//! command (address, optional first-message auth signature, and the
//! viewer's agent arguments). The `"viewer"` lock is a per-state
//! singleton marker (content `"ready"`).

use envconfig::Envconfig;
use objectiveai_sdk::cli::command::websocket::WebSocketExecutor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify;

use crate::plugins::serve_plugin_asset;

#[tauri::command]
fn viewer_ready(state: tauri::State<'_, Arc<Notify>>) {
    state.notify_one();
}

/// Everything the JS frontend's own WebSocket clients take: the
/// published base `ws://` address, the optional first-message auth
/// signature, and the agent arguments identifying viewer-initiated
/// executions. The frontend appends `/listen` / `/execute` and
/// connects with native WebSockets — no daemon traffic flows through
/// the Rust side.
#[derive(Clone, serde::Serialize)]
pub struct WebSocketConfig {
    pub address: String,
    pub signature: Option<String>,
    pub agent_arguments: objectiveai_sdk::cli::command::AgentArguments,
}

#[tauri::command]
fn websocket_config(state: tauri::State<'_, WebSocketConfig>) -> WebSocketConfig {
    state.inner().clone()
}

/// The per-state agents root the `client` remote links open:
/// `<objectiveai_dir>/state/<state>/agents`.
pub struct AgentsDir(pub PathBuf);

/// Open a `client` remote's local folder
/// (`<agents root>/<owner>/<repository>`) in the OS file manager.
/// Path segments are rejected if they'd escape the agents root.
#[tauri::command]
fn open_agent_remote(
    state: tauri::State<'_, AgentsDir>,
    owner: String,
    repository: String,
) -> Result<(), String> {
    for segment in [&owner, &repository] {
        if segment.is_empty()
            || segment.contains(['/', '\\'])
            || segment == "."
            || segment == ".."
        {
            return Err(format!("invalid remote path segment: {segment:?}"));
        }
    }
    let path = state.0.join(&owner).join(&repository);
    open::that_detached(&path).map_err(|e| e.to_string())
}

/// Open a GitHub URL in the default browser. Restricted to
/// `https://github.com/` so the frontend can't shell-open arbitrary
/// targets.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/") {
        return Err("only https://github.com/ urls may be opened".to_string());
    }
    open::that_detached(&url).map_err(|e| e.to_string())
}

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "DAEMON_ADDRESS")]
    daemon_address: Option<String>,
    #[envconfig(from = "DAEMON_SIGNATURE")]
    daemon_signature: Option<String>,
    #[envconfig(from = "SUPPRESS_OUTPUT")]
    suppress_output: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_DIR")]
    objectiveai_dir: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_STATE")]
    objectiveai_state: Option<String>,
}

impl EnvConfigBuilder {
    pub fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            daemon_address: self.daemon_address,
            daemon_signature: self.daemon_signature,
            suppress_output: self
                .suppress_output
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")),
            objectiveai_dir: self.objectiveai_dir,
            objectiveai_state: self.objectiveai_state,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub daemon_address: Option<String>,
    pub daemon_signature: Option<String>,
    pub suppress_output: Option<bool>,
    pub objectiveai_dir: Option<String>,
    pub objectiveai_state: Option<String>,
}

impl Envconfig for ConfigBuilder {
    #[allow(deprecated)]
    fn init() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init().map(|e| e.build())
    }

    fn init_from_env() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_env().map(|e| e.build())
    }

    fn init_from_hashmap(hashmap: &std::collections::HashMap<String, String>) -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_hashmap(hashmap).map(|e| e.build())
    }
}

impl ConfigBuilder {
    pub fn build(self) -> Config {
        Config {
            daemon_address: self.daemon_address,
            daemon_signature: self.daemon_signature,
            suppress_output: self.suppress_output.unwrap_or(false),
            // Layout root (OBJECTIVEAI_DIR). Same default as the api.
            objectiveai_dir: match self.objectiveai_dir {
                Some(dir) => PathBuf::from(dir),
                None => dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".objectiveai"),
            },
            objectiveai_state: self
                .objectiveai_state
                .unwrap_or_else(|| "default".to_string()),
        }
    }
}

pub struct Config {
    /// The daemon's `ws://` connect URL (`DAEMON_ADDRESS`). REQUIRED —
    /// [`run`] errors out when unset. Provided by `objectiveai viewer
    /// spawn`, which resolves it from the daemon it just ensured.
    /// `Option` only so `ConfigBuilder::build` stays infallible.
    pub daemon_address: Option<String>,
    /// Optional daemon WebSocket auth signature
    /// (`DAEMON_SIGNATURE`): the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>` sent verbatim in the
    /// first-message auth preamble on every connection. `None` =
    /// connect unauthenticated (the daemon must be open).
    pub daemon_signature: Option<String>,
    pub suppress_output: bool,
    /// Layout root (`OBJECTIVEAI_DIR`); default `~/.objectiveai`.
    pub objectiveai_dir: PathBuf,
    /// State name (`OBJECTIVEAI_STATE`); default `"default"`.
    pub objectiveai_state: String,
}

/// The one Rust-side WebSocket executor: `list_plugins_with_viewer`
/// discovers plugins through it at startup. Everything else is
/// JS-native. Commands travel to the daemon's `/execute` route and
/// run in-process there — the viewer never spawns the cli binary, so
/// it can live on a different machine than the CLI. `daemon_address`
/// is the daemon's published base `ws://` URL (the same one the JS
/// frontend connects to).
pub fn make_executor(daemon_address: &str, signature: Option<&str>) -> WebSocketExecutor {
    let executor = WebSocketExecutor::new(format!("{daemon_address}/execute"));
    match signature {
        Some(signature) => executor.signature(signature),
        None => executor,
    }
}

/// Resolve the shell's supporting state. No IO.
pub fn setup(config: &Config) -> PathBuf {
    crate::plugins::plugins_dir(&config.objectiveai_dir)
}

/// A function that exits the viewer's event loop with the given exit code.
pub type Exiter = Box<dyn FnOnce(i32) + Send>;

/// Must be called on the main thread. Tauri's event loop panics otherwise.
/// Spawn `setup` and other async work on tokio tasks instead.
///
/// If `exiter_tx` is provided, an `Exiter` is sent through it once
/// Tauri is initialized. Call the exiter from a spawned task
/// to make `serve` return.
///
/// Returns the exit code from Tauri's event loop.
pub fn serve(
    executor: WebSocketExecutor,
    websocket_config_state: WebSocketConfig,
    agents_dir: AgentsDir,
    plugins_dir: PathBuf,
    exiter_tx: Option<tokio::sync::oneshot::Sender<Exiter>>,
) -> i32 {
    // `viewer_ready`'s readiness marker. Nothing consumes the
    // notification today (the JS frontend talks to the daemon
    // directly); the command is kept as a startup signal for later.
    let ready = Arc::new(Notify::new());

    let plugins_dir_for_protocol = plugins_dir.clone();
    let builder = tauri::Builder::default()
        .manage(ready)
        .manage(executor)
        .manage(websocket_config_state)
        .manage(agents_dir)
        .manage(crate::plugins::PluginsDir(plugins_dir))
        .register_uri_scheme_protocol("plugin", move |_app, request| {
            serve_plugin_asset(&plugins_dir_for_protocol, request)
        });
    let builder = builder.invoke_handler(tauri::generate_handler![
        viewer_ready,
        websocket_config,
        open_agent_remote,
        open_url,
        crate::plugins::list_plugins_with_viewer,
    ]);
    builder
        .setup(move |tauri_app| {
            if let Some(tx) = exiter_tx {
                let exit_handle = tauri_app.handle().clone();
                tx.send(Box::new(move |code| exit_handle.exit(code))).ok();
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application")
        .run_return(|_, _| {})
}

/// Sets up and serves the viewer. Returns the exit code from Tauri's event loop.
/// The caller should use `std::process::exit(code)` with the returned value.
pub async fn run(config: Config) -> std::io::Result<i32> {
    // The daemon address is the viewer's one data source — refuse to
    // start without it rather than sitting dark.
    let daemon_address = config.daemon_address.clone().ok_or_else(|| {
        std::io::Error::other(
            "DAEMON_ADDRESS is not set — start the viewer via `objectiveai viewer spawn`, \
             which passes the daemon's ws:// address",
        )
    })?;

    let lock_dir = config
        .objectiveai_dir
        .join("state")
        .join(&config.objectiveai_state)
        .join("locks");
    let plugins_dir = setup(&config);
    let executor = make_executor(&daemon_address, config.daemon_signature.as_deref());

    // There is only ever ONE viewer per STATE (unlike the api, which
    // is one per OBJECTIVEAI_DIR): claim key "viewer" in
    // <dir>/state/<state>/locks. The viewer is a WebSocket client (no
    // listener), so the content is a plain readiness marker, not a
    // URL. The claim is held until process death (LockClaim leaks on
    // drop by design) and the kernel releases it on any exit, crash
    // included.
    if objectiveai_sdk::lockfile::try_acquire(&lock_dir, "viewer", "ready")
        .await
        .is_none()
    {
        return Err(std::io::Error::other(
            "another objectiveai-viewer instance already holds the viewer lock for this state",
        ));
    }

    // No Rust-side daemon streams: the JS frontend connects to the
    // daemon directly with native WebSockets, using the variables the
    // `websocket_config` command hands it — the same viewer identity
    // the Rust-side executor stamps on its own calls.
    let websocket_config_state = WebSocketConfig {
        address: daemon_address,
        signature: config.daemon_signature.clone(),
        agent_arguments: crate::plugins::viewer_agent_arguments(),
    };

    let agents_dir = AgentsDir(
        config
            .objectiveai_dir
            .join("state")
            .join(&config.objectiveai_state)
            .join("agents"),
    );

    Ok(serve(
        executor,
        websocket_config_state,
        agents_dir,
        plugins_dir,
        None,
    ))
}
