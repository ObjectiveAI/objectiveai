//! Viewer lifecycle: env config, the event bus, and the Tauri shell.
//!
//! The Rust side owns ALL daemon traffic: the JS frontend never sees
//! the daemon's address, auth signature, or the viewer's agent
//! identity — every daemon stream rides the
//! [`crate::daemon_proxy`] Tauri commands over IPC channels (the
//! webview's per-origin HTTP connection cap starved the old
//! direct-fetch model). The `"viewer"` lock is a per-state singleton
//! marker (content `"ready"`).

use envconfig::Envconfig;
use objectiveai_sdk::cli::command::sse::SseCommandExecutor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify;

use crate::plugins::serve_plugin_asset;

#[tauri::command]
fn viewer_ready(state: tauri::State<'_, Arc<Notify>>) {
    state.notify_one();
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

/// Create — or focus, when already open — the agent conversation
/// window for one AIH: the `agent.html` entry (the popup UI as a full
/// window; no tabs, no footer). The AIH reaches the page via an
/// initialization script (a global set before any page script runs) —
/// NOT a URL query: `WebviewUrl::App` is a PathBuf, so a query string
/// would be treated as part of the asset path and 404 to a white
/// window.
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
    let aih_json = serde_json::to_string(agent_instance_hierarchy)
        .expect("a str serializes infallibly");
    tauri::WebviewWindowBuilder::new(
        app,
        &label,
        tauri::WebviewUrl::App("agent.html".into()),
    )
    .initialization_script(format!(
        "window.__AGENT_INSTANCE_HIERARCHY__ = {aih_json};"
    ))
    .title(agent_instance_hierarchy)
    .inner_size(1024.0, 768.0)
    .build()?;
    Ok(())
}

/// Open (or focus) the agent conversation window for `aih` — the tree's
/// explicit `open` chip calls this instead of an in-page popup.
///
/// ASYNC on purpose: a sync command runs on the MAIN thread, and
/// webview creation on Windows contends with that same event loop —
/// the window shell appears but the page never initializes (a white
/// window). An async command runs off the main thread, so the
/// creation dispatches cleanly.
#[tauri::command]
async fn open_agent_window(app: tauri::AppHandle, aih: String) -> Result<(), String> {
    open_agent_window_impl(&app, &aih).map_err(|e| e.to_string())
}

/// The deterministic-within-one-process window label for one
/// laboratory. Lab ids are only unique per (machine, machine_state),
/// so all three feed the hash; `Hash for str` is length-prefixed, so
/// sequential hashing needs no separators.
fn laboratory_window_label(
    id: &str,
    machine: Option<&str>,
    machine_state: Option<&str>,
) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    machine.unwrap_or("").hash(&mut hasher);
    machine_state.unwrap_or("").hash(&mut hasher);
    format!("laboratory-{:016x}", hasher.finish())
}

/// Create — or focus, when already open — the laboratory filesystem
/// window for one laboratory: the `laboratory.html` entry. The
/// identity reaches the page via an initialization script (a global
/// set before any page script runs) — NOT a URL query:
/// `WebviewUrl::App` is a PathBuf, so a query string would be treated
/// as part of the asset path and 404 to a white window (same as
/// [`open_agent_window_impl`]).
fn open_laboratory_window_impl(
    app: &tauri::AppHandle,
    id: &str,
    machine: Option<&str>,
    machine_state: Option<&str>,
    machine_os: Option<&str>,
) -> tauri::Result<()> {
    use tauri::Manager;
    let label = laboratory_window_label(id, machine, machine_state);
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.set_focus();
        return Ok(());
    }
    let global = serde_json::json!({
        "id": id,
        "machine": machine,
        "machineState": machine_state,
    })
    .to_string();
    // `{os}/{machine id}/{lab id}` — the serving machine's identity as
    // the window's name, unknown segments rendered as `?`.
    let title = format!(
        "{}/{}/{id}",
        machine_os.unwrap_or("?"),
        machine.unwrap_or("?"),
    );
    tauri::WebviewWindowBuilder::new(
        app,
        &label,
        tauri::WebviewUrl::App("laboratory.html".into()),
    )
    .initialization_script(format!("window.__LABORATORY__ = {global};"))
    .title(title)
    .inner_size(1024.0, 768.0)
    .build()?;
    Ok(())
}

/// Open (or focus) the laboratory filesystem window — the laboratory
/// card's `open` tab calls this. ASYNC on purpose, same as
/// [`open_agent_window`]: a sync command white-screens webview
/// creation on Windows.
#[tauri::command]
async fn open_laboratory_window(
    app: tauri::AppHandle,
    id: String,
    machine: Option<String>,
    machine_state: Option<String>,
    machine_os: Option<String>,
) -> Result<(), String> {
    open_laboratory_window_impl(
        &app,
        &id,
        machine.as_deref(),
        machine_state.as_deref(),
        machine_os.as_deref(),
    )
    .map_err(|e| e.to_string())
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
    /// The daemon's `http://` connect URL (`DAEMON_ADDRESS`). REQUIRED —
    /// [`run`] errors out when unset. Provided by `objectiveai viewer
    /// spawn`, which resolves it from the daemon it just ensured.
    /// `Option` only so `ConfigBuilder::build` stays infallible.
    pub daemon_address: Option<String>,
    /// Optional daemon auth signature (`DAEMON_SIGNATURE`): the
    /// pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>` sent as the
    /// `X-OBJECTIVEAI-SIGNATURE` header on every request. `None` =
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

/// The one Rust-side command executor: `list_plugins_with_viewer`
/// discovers plugins through it at startup. Everything else is
/// JS-native. Commands travel to the daemon's `/execute` route and
/// run in-process there — the viewer never spawns the cli binary, so
/// it can live on a different machine than the CLI. `daemon_address`
/// is the daemon's published base `http://` URL (the same one the JS
/// frontend connects to).
pub fn make_executor(daemon_address: &str, signature: Option<&str>) -> SseCommandExecutor {
    let executor = SseCommandExecutor::new(format!("{daemon_address}/execute"));
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
    executor: SseCommandExecutor,
    proxy: crate::daemon_proxy::DaemonProxy,
    agents_dir: AgentsDir,
    plugins_dir: PathBuf,
    lab_env: crate::laboratories::LabEnv,
    exiter_tx: Option<tokio::sync::oneshot::Sender<Exiter>>,
    agent_window: Option<String>,
) -> i32 {
    // `viewer_ready`'s readiness marker. Nothing consumes the
    // notification today; the command is kept as a startup signal
    // for later.
    let ready = Arc::new(Notify::new());

    let plugins_dir_for_protocol = plugins_dir.clone();
    let builder = tauri::Builder::default()
        .manage(ready)
        .manage(executor)
        .manage(proxy)
        .manage(agents_dir)
        .manage(lab_env)
        .manage(crate::plugins::PluginsDir(plugins_dir))
        .register_uri_scheme_protocol("plugin", move |_app, request| {
            serve_plugin_asset(&plugins_dir_for_protocol, request)
        });
    let builder = builder.invoke_handler(tauri::generate_handler![
        viewer_ready,
        open_agent_remote,
        open_url,
        open_agent_window,
        open_laboratory_window,
        crate::daemon_proxy::daemon_listen,
        crate::daemon_proxy::daemon_execute,
        crate::daemon_proxy::daemon_agents_instances_list,
        crate::daemon_proxy::daemon_agents_instance,
        crate::daemon_proxy::daemon_laboratories_list,
        crate::daemon_proxy::daemon_laboratory,
        crate::daemon_proxy::daemon_laboratory_filetree,
        crate::daemon_proxy::daemon_stream_close,
        crate::plugins::list_plugins_with_viewer,
        crate::laboratories::machine_identity,
        crate::laboratories::laboratories_spawn_host,
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
             which passes the daemon's http:// address",
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
    // <dir>/state/<state>/locks. The viewer is an HTTP client of the daemon (no
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

    // ALL daemon streams flow through the Rust-side proxy commands
    // (`crate::daemon_proxy`) — the JS frontend never holds the
    // address or signature.
    let proxy = crate::daemon_proxy::DaemonProxy::new(
        daemon_address,
        config.daemon_signature.clone(),
    );

    let agents_dir = AgentsDir(
        config
            .objectiveai_dir
            .join("state")
            .join(&config.objectiveai_state)
            .join("agents"),
    );

    let lab_env = crate::laboratories::LabEnv {
        objectiveai_dir: config.objectiveai_dir.clone(),
        state: config.objectiveai_state.clone(),
    };

    Ok(serve(
        executor,
        proxy,
        agents_dir,
        plugins_dir,
        lab_env,
        None,
        config.agent_instance_hierarchy.clone(),
    ))
}
