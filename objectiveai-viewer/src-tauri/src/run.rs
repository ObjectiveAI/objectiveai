//! Viewer lifecycle: env config, the event bus, the Tauri shell, and
//! the daemon WebSocket client.
//!
//! The viewer is a WebSocket CLIENT of the CLI daemon's broadcast —
//! not a server. It holds no listener and exposes no routes: the
//! [`crate::daemon_ws`] task connects to the daemon's published
//! `ws://` endpoint (optional `DAEMON_SIGNATURE` auth) and forwards
//! every frame onto the same mpsc event bus the Tauri shell drains to
//! the JS side. The `"viewer"` lock is a per-state singleton marker
//! (content `"ready"`), no longer a connect URL.

use envconfig::Envconfig;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{Notify, mpsc};

use crate::plugins::serve_plugin_asset;
use objectiveai_sdk::viewer::{Event, EventReceiver};

#[tauri::command]
fn viewer_ready(state: tauri::State<'_, Arc<Notify>>) {
    state.notify_one();
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
    /// Optional daemon WebSocket auth header value
    /// (`DAEMON_SIGNATURE`): the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>` sent verbatim as
    /// `X-DAEMON-SIGNATURE` on every upgrade. `None` = connect
    /// unauthenticated (the daemon must be open).
    pub daemon_signature: Option<String>,
    pub suppress_output: bool,
    /// Layout root (`OBJECTIVEAI_DIR`); default `~/.objectiveai`.
    pub objectiveai_dir: PathBuf,
    /// State name (`OBJECTIVEAI_STATE`); default `"default"`.
    pub objectiveai_state: String,
}

/// One cli-binary executor, stamped with the layout coordinates so
/// every spawned child resolves the same tree the viewer serves
/// `plugin://` assets from — even when the viewer's own config came
/// from a programmatic `ConfigBuilder` rather than the env.
fn make_executor(config: &Config) -> BinaryExecutor {
    BinaryExecutor::new(Some(config.objectiveai_dir.clone()))
        .env(
            "OBJECTIVEAI_DIR",
            config.objectiveai_dir.to_string_lossy().into_owned(),
        )
        .env("OBJECTIVEAI_STATE", config.objectiveai_state.clone())
}

/// Build the event bus and the shell's supporting state. No IO — the
/// daemon WebSocket client (the bus's producer) is spawned separately
/// by [`run`], so embedders can drive `setup` + `serve` with synthetic
/// events only.
pub fn setup(
    config: &Config,
) -> (
    objectiveai_sdk::viewer::EventSender,
    EventReceiver,
    BinaryExecutor,
    PathBuf,
) {
    let (tx, rx) = mpsc::unbounded_channel::<Event>();

    // One executor for everything the viewer runs through the cli
    // binary: `cli_run` dispatches from plugin iframes and
    // `list_plugins_with_viewer` from the shell.
    let executor = make_executor(config);
    let plugins_dir = crate::plugins::plugins_dir(&config.objectiveai_dir);

    (tx, rx, executor, plugins_dir)
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
    events_tx: objectiveai_sdk::viewer::EventSender,
    mut rx: EventReceiver,
    executor: BinaryExecutor,
    plugins_dir: PathBuf,
    exiter_tx: Option<tokio::sync::oneshot::Sender<Exiter>>,
) -> i32 {
    let ready = Arc::new(Notify::new());
    let ready_for_task = ready.clone();

    let plugins_dir_for_protocol = plugins_dir.clone();
    let builder = tauri::Builder::default()
        .manage(ready)
        .manage(executor)
        .manage(events_tx)
        .manage(crate::plugins::PluginsDir(plugins_dir))
        .register_uri_scheme_protocol("plugin", move |_app, request| {
            serve_plugin_asset(&plugins_dir_for_protocol, request)
        });
    let builder = builder.invoke_handler(tauri::generate_handler![
        viewer_ready,
        crate::plugins::list_plugins_with_viewer,
        crate::cli_command::cli_execute,
    ]);
    builder
        .setup(move |tauri_app| {
            let handle = tauri_app.handle().clone();
            if let Some(tx) = exiter_tx {
                let exit_handle = handle.clone();
                tx.send(Box::new(move |code| exit_handle.exit(code))).ok();
            }
            tauri::async_runtime::spawn(async move {
                // Buffer events until the frontend signals it is listening.
                let mut buffer = Vec::new();
                loop {
                    tokio::select! {
                        biased;
                        _ = ready_for_task.notified() => break,
                        event = rx.recv() => {
                            match event {
                                Some(e) => buffer.push(e),
                                None => return,
                            }
                        }
                    }
                }
                // Drain buffered events.
                for event in buffer {
                    let _ = handle.emit(event.destination(), &event);
                }
                // Forward remaining events directly.
                while let Some(event) = rx.recv().await {
                    let _ = handle.emit(event.destination(), &event);
                }
            });
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
    let (events_tx, rx, executor, plugins_dir) = setup(&config);

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

    // The event bus's producer: connect to the daemon's broadcast
    // WebSocket at the address the spawner handed us and forward every
    // frame to the JS side.
    crate::daemon_ws::spawn_client(
        events_tx.clone(),
        daemon_address,
        config.daemon_signature.clone(),
    );

    Ok(serve(events_tx, rx, executor, plugins_dir, None))
}
