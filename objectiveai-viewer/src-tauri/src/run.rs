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
    exiter_tx: Option<tokio::sync::oneshot::Sender<Exiter>>,
    agent_window: Option<String>,
) -> i32 {
    // `viewer_ready`'s readiness marker. Nothing consumes the
    // notification today; the command is kept as a startup signal
    // for later.
    let ready = Arc::new(Notify::new());

    // The tab registry — seeded BEFORE the shell boots so the main
    // window's first snapshot already holds its tabs. Normal boot:
    // the two home tabs. `--agent-instance-hierarchy`: one agent
    // conversation tab (a scoped debug instance; closing it leaves
    // the main empty state).
    let registry = crate::tabs::TabRegistry::default();
    match &agent_window {
        Some(aih) => registry.seed(
            "main",
            vec![crate::tabs::TabKind::Agent { aih: aih.clone() }],
        ),
        None => registry.seed(
            "main",
            vec![
                crate::tabs::TabKind::Agents,
                crate::tabs::TabKind::Laboratories,
            ],
        ),
    }
    let main_title = agent_window
        .clone()
        .unwrap_or_else(|| "ObjectiveAI Viewer".to_string());

    let builder = tauri::Builder::default()
        .manage(ready)
        .manage(proxy)
        .manage(agents_dir)
        .manage(lab_env)
        .manage(registry);
    let builder = builder.invoke_handler(tauri::generate_handler![
        viewer_ready,
        open_agent_remote,
        open_url,
        crate::tabs::tabs_snapshot,
        crate::tabs::tabs_open,
        crate::tabs::tabs_select,
        crate::tabs::tabs_close,
        crate::tabs::tabs_move,
        crate::tabs::tabs_detach,
        crate::daemon_proxy::daemon_listen,
        crate::daemon_proxy::daemon_execute,
        crate::daemon_proxy::daemon_agents_instances_list,
        crate::daemon_proxy::daemon_agents_instance,
        crate::daemon_proxy::daemon_laboratories_list,
        crate::daemon_proxy::daemon_laboratory,
        crate::daemon_proxy::daemon_laboratory_filetree,
        crate::daemon_proxy::daemon_user,
        crate::daemon_proxy::daemon_user_reply,
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
            crate::docking::spawn(
                tauri_app.handle().clone(),
                dock_rx.take().expect("setup runs once"),
            );
            // Windows are created HERE, not in tauri.conf.json. Every
            // window — main included — runs the SAME shell entry; the
            // registry decides what it shows.
            tauri::WebviewWindowBuilder::new(
                tauri_app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title(&main_title)
            .inner_size(1024.0, 768.0)
            .build()?;
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
                // The main window IS the app: closing it takes every
                // shell window (and the process) with it.
                tauri::WindowEvent::CloseRequested { .. } => {
                    if label == "main" {
                        app_handle.exit(0);
                    }
                }
                // A window died (user close, tabs_close auto-close,
                // dock merge, teardown) — drop its registry slice.
                // Idempotent, and every Result is ignored: this also
                // runs during app teardown.
                tauri::WindowEvent::Destroyed => {
                    let registry = app_handle.state::<crate::tabs::TabRegistry>();
                    if registry.remove_window(&label) {
                        use tauri::Emitter;
                        let snapshot = registry.snapshot();
                        let _ = app_handle.emit("tabs://changed", &snapshot);
                    }
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

    let lock_dir = config
        .objectiveai_dir
        .join("state")
        .join(&config.objectiveai_state)
        .join("locks");

    // There is only ever ONE viewer per STATE (unlike the api, which
    // is one per OBJECTIVEAI_DIR): claim key "viewer" in
    // <dir>/state/<state>/locks. The viewer is an HTTP client of the daemon (no
    // listener), so the content is a plain readiness marker, not a
    // URL. The claim is held until process death (LockClaim leaks on
    // drop by design) and the kernel releases it on any exit, crash
    // included. An `--agent-instance-hierarchy` instance is a SCOPED
    // debug window, not THE viewer — it takes no lock and coexists
    // with a running main viewer.
    if config.agent_instance_hierarchy.is_none() {
        // Readiness handshake: the daemon (this viewer's spawner and
        // leash-holder) blocks on this stdout line. No address — the
        // viewer is a client, not a server. No lockfile: the daemon
        // owns this process's lifetime outright. Scoped debug windows
        // (--agent-instance-hierarchy) stay silent — they are not THE
        // viewer.
        objectiveai_sdk::process::print_ready(None);
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
        proxy,
        agents_dir,
        lab_env,
        None,
        config.agent_instance_hierarchy.clone(),
    ))
}
