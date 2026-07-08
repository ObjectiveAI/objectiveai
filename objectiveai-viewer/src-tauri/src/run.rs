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

/// The deterministic-within-one-process window label for an AIH —
/// labels must be alphanumeric/`-`/`_` and AIHs contain `/`, so the
/// label is a hash; create-or-focus keys on it.
fn agent_window_label(agent_instance_hierarchy: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    agent_instance_hierarchy.hash(&mut hasher);
    format!("agent-{:016x}", hasher.finish())
}

/// Percent-encode one URL query component (RFC 3986 unreserved set
/// passes through; everything else — `/`, `&`, `#`, spaces, UTF-8
/// bytes — encodes).
fn encode_query_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Create — or focus, when already open — the agent conversation
/// window for one AIH: the `agent.html` entry (the popup UI as a full
/// window; no tabs, no footer), scoped by the `aih` query parameter.
fn open_agent_window_impl(
    app: &tauri::AppHandle,
    agent_instance_hierarchy: &str,
) -> tauri::Result<()> {
    use tauri::Manager;
    let label = agent_window_label(agent_instance_hierarchy);
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.set_focus();
        return Ok(());
    }
    let url = format!(
        "agent.html?aih={}",
        encode_query_component(agent_instance_hierarchy),
    );
    tauri::WebviewWindowBuilder::new(app, &label, tauri::WebviewUrl::App(url.into()))
        .title(agent_instance_hierarchy)
        .inner_size(1024.0, 768.0)
        .build()?;
    Ok(())
}

/// Open (or focus) the agent conversation window for `aih` — the tree's
/// explicit `open` chip calls this instead of an in-page popup.
#[tauri::command]
fn open_agent_window(app: tauri::AppHandle, aih: String) -> Result<(), String> {
    open_agent_window_impl(&app, &aih).map_err(|e| e.to_string())
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
            agent_instance_hierarchy: None,
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
    /// `--agent-instance-hierarchy`: open ONLY the agent conversation
    /// window for this AIH (the main window never opens, and the
    /// per-state viewer singleton lock is NOT taken — a scoped debug
    /// instance, not THE viewer). Set by `main.rs` from clap, not the
    /// environment.
    pub agent_instance_hierarchy: Option<String>,
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
    agent_window: Option<String>,
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
        open_agent_window,
        crate::plugins::list_plugins_with_viewer,
    ]);
    builder
        .setup(move |tauri_app| {
            if let Some(tx) = exiter_tx {
                let exit_handle = tauri_app.handle().clone();
                tx.send(Box::new(move |code| exit_handle.exit(code))).ok();
            }
            // Windows are created HERE, not in tauri.conf.json —
            // `--agent-instance-hierarchy` opens ONLY that agent's
            // conversation window; otherwise the main window opens.
            match &agent_window {
                Some(aih) => {
                    open_agent_window_impl(tauri_app.handle(), aih)?;
                }
                None => {
                    tauri::WebviewWindowBuilder::new(
                        tauri_app,
                        "main",
                        tauri::WebviewUrl::App("index.html".into()),
                    )
                    .title("ObjectiveAI Viewer")
                    .inner_size(1024.0, 768.0)
                    .build()?;
                }
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
    // included. An `--agent-instance-hierarchy` instance is a SCOPED
    // debug window, not THE viewer — it takes no lock and coexists
    // with a running main viewer.
    if config.agent_instance_hierarchy.is_none()
        && objectiveai_sdk::lockfile::try_acquire(&lock_dir, "viewer", "ready")
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
        config.agent_instance_hierarchy.clone(),
    ))
}
