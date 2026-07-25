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
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify;

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
    proxy: crate::daemon_proxy::DaemonProxy,
    agents_dir: AgentsDir,
    lab_env: crate::laboratories::LabEnv,
    log_sink: crate::shell::LogSink,
    command_log_sink: crate::shell::CommandLogSink,
    tab_inventory: crate::shell::TabInventory,
    plugins_dirs: crate::shell::PluginsDirs,
    exiter_tx: Option<tokio::sync::oneshot::Sender<Exiter>>,
) -> i32 {
    let plugins_root = plugins_dirs.plugins_root();
    let plugins_temp = plugins_dirs.temp_dir();
    // `viewer_ready`'s readiness marker. Nothing consumes the
    // notification today; the command is kept as a startup signal
    // for later.
    let ready = Arc::new(Notify::new());

    // The shell model — one EMPTY boot window. Rust seeds no tabs
    // and knows no tab names: the boot chrome opens the home tabs
    // through `tabs_open`, the same API every identity (plugins
    // included, later) uses. The boot window is an ORDINARY shell
    // window (no window is special; the app lives exactly as long as
    // windows exist).
    let (model, boot_label) = crate::shell::ShellModel::boot();

    let builder = tauri::Builder::default()
        .manage(ready)
        .manage(proxy)
        .manage(agents_dir)
        .manage(lab_env)
        .manage(model)
        .manage(crate::shell::WebviewSync::default())
        .manage(log_sink)
        .manage(command_log_sink)
        .manage(tab_inventory)
        .manage(plugins_dirs)
        .manage(crate::shell::ChannelRequests::new(plugins_root.clone()));
    let builder = builder.invoke_handler(tauri::generate_handler![
        viewer_ready,
        open_agent_remote,
        open_url,
        crate::shell::tabs_snapshot,
        crate::shell::tabs_open,
        crate::shell::tab_self,
        crate::shell::tabs_declare,
        crate::shell::tabs_inventory,
        crate::shell::tabs_toggle,
        crate::shell::tabs_reorder,
        crate::shell::plugins_list,
        crate::shell::plugins_install,
        crate::shell::plugins_uninstall,
        crate::shell::tabs_select,
        crate::shell::tabs_close,
        crate::shell::tabs_close_self,
        crate::shell::channel_request_declare,
        crate::shell::channel_request_accept,
        crate::shell::tabs_move,
        crate::shell::tabs_detach,
        crate::shell::ui_set,
        crate::shell::ui_get,
        crate::shell::logs_report,
        crate::shell::logs_pull,
        crate::shell::command_logs_pull,
        crate::shell::command_log_items_pull,
        crate::daemon_proxy::daemon_listen,
        crate::daemon_proxy::daemon_execute,
        crate::daemon_proxy::daemon_agents_instances_list,
        crate::daemon_proxy::daemon_agents_instance,
        crate::daemon_proxy::daemon_laboratories_list,
        crate::daemon_proxy::daemon_laboratory,
        crate::daemon_proxy::daemon_laboratory_filetree,
        crate::daemon_proxy::daemon_channels,
        crate::daemon_proxy::daemon_channel_accept,
        crate::daemon_proxy::daemon_viewer_plugin,
        crate::daemon_proxy::daemon_stream_close,
        crate::laboratories::machine_identity,
    ]);
    // The docking task's Moved feed — the run_return closure is the
    // producer; the task (spawned in setup, where an AppHandle
    // exists) is the consumer.
    let (dock_tx, dock_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let mut dock_rx = Some(dock_rx);
    let app = builder
        .setup(move |tauri_app| {
            if let Some(tx) = exiter_tx {
                let exit_handle = tauri_app.handle().clone();
                tx.send(Box::new(move |code| exit_handle.exit(code))).ok();
            }
            crate::shell::spawn_docking(
                tauri_app.handle().clone(),
                dock_rx.take().expect("setup runs once"),
            );
            // The resident /listen capture — Rust holds the daemon
            // stream for the viewer's whole life; the command-logs
            // tab is just a view over what it writes.
            crate::shell::spawn_command_listener(tauri_app.handle().clone());
            // Sweep the viewer's OWN temp partition (a hard-killed
            // predecessor's install scratch) — installs can't start
            // before a chrome webview can invoke, so nothing races.
            tauri::async_runtime::spawn(async move {
                objectiveai_sdk::gitrepo::sweep_temp(&plugins_temp).await;
            });
            // The resident /channels listener — every incoming offer
            // spawns a detached channel-request window.
            crate::shell::spawn_channel_listener(
                tauri_app.handle().clone(),
                plugins_root.clone(),
            );
            // Windows are created HERE, not in tauri.conf.json. Every
            // window is a raw Window + a chrome webview (strip +
            // status bar); the model decides which tab webviews it
            // hosts — the spawned sync seeds the boot tabs' content
            // webviews.
            crate::shell::build_shell_window(
                tauri_app.handle(),
                &boot_label,
                "ObjectiveAI Viewer",
                None,
            )?;
            // The boot orchestrator: await the chrome's root-tab
            // declaration + the bin/plugins scan, then open every
            // ENABLED inventory tab into the boot window.
            crate::shell::spawn_boot_orchestrator(
                tauri_app.handle().clone(),
                plugins_root,
                boot_label.clone(),
            );
            let handle = tauri_app.handle().clone();
            tauri::async_runtime::spawn(async move {
                crate::shell::sync(&handle).await;
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application");
    app.run_return(move |app_handle, event| {
        use tauri::Manager;
        if let tauri::RunEvent::WindowEvent { label, event, .. } = event {
            match event {
                // Feed the docking task. Send errors (task gone at
                // teardown) are meaningless.
                tauri::WindowEvent::Moved(_) => {
                    let _ = dock_tx.send(label);
                }
                // A window died (user close, tabs_close auto-close,
                // dock merge, teardown) — drop its model slice, then
                // reconcile (belt-and-braces: its content webviews
                // normally die with the parent HWND). Spawned: this
                // closure is synchronous on the runtime's thread and
                // the model lock is a tokio mutex. Idempotent, every
                // Result ignored — this also runs during teardown.
                //
                // No window is special, so the app's lifetime is
                // simply "while windows exist": the LAST window's
                // death exits the process. (Detach can't hit this —
                // a sole tab drags its window whole; dock closes the
                // source only after the target proved alive.)
                tauri::WindowEvent::Destroyed => {
                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let model = handle.state::<crate::shell::ShellModel>();
                        if let Some(snapshot) = model.remove_window(&label).await {
                            crate::shell::publish(&handle, &snapshot, &[]);
                            crate::shell::sync(&handle).await;
                        }
                        if handle.windows().is_empty() {
                            handle.exit(0);
                        }
                    });
                }
                // Keep the content webviews SIZED to the content
                // band. Inline + size-only: positions (active and
                // parked alike) are constants that resize never
                // changes, so no model read and no dispatch round
                // trips — set_size fast-paths on this thread.
                tauri::WindowEvent::Resized(_)
                | tauri::WindowEvent::ScaleFactorChanged { .. } => {
                    crate::shell::layout_window(app_handle, &label);
                }
                _ => {}
            }
        }
    })
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

    // Readiness handshake: the daemon (this viewer's spawner and
    // leash-holder) blocks on this stdout line. No address — the
    // viewer is a client of the daemon, not a server; the daemon owns
    // this process's lifetime outright.
    objectiveai_sdk::process::print_ready(None);

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

    // This run's logfile sinks — timestamped at viewer start, under
    // the state folder: <dir>/state/<state>/viewer/{viewer-logs,
    // command-logs}.
    let viewer_dir = config
        .objectiveai_dir
        .join("state")
        .join(&config.objectiveai_state)
        .join("viewer");
    let log_sink = crate::shell::LogSink::new(viewer_dir.join("viewer-logs"));
    let command_log_sink =
        crate::shell::CommandLogSink::new(viewer_dir.join("command-logs"));
    // Persisted tab toggles, beside the log sinks in the state
    // folder.
    let tab_inventory = crate::shell::TabInventory::new(viewer_dir.join("tabs.json"));

    // The installer's directory layout (the installed-plugin tree
    // lives inside it, machine-wide, shared across states).
    let plugins_dirs = crate::shell::PluginsDirs::new(config.objectiveai_dir.clone());

    Ok(serve(
        proxy,
        agents_dir,
        lab_env,
        log_sink,
        command_log_sink,
        tab_inventory,
        plugins_dirs,
        None,
    ))
}
