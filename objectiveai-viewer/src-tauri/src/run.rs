use axum::Json;
use axum::http::StatusCode;
use axum::middleware;
use envconfig::Envconfig;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{Notify, mpsc};

use objectiveai_sdk::viewer::{Event, EventReceiver};
use crate::plugins::{register_plugin_route, serve_plugin_asset};
use crate::signature::signature_middleware;

#[tauri::command]
fn viewer_ready(state: tauri::State<'_, Arc<Notify>>) {
    state.notify_one();
}

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "VIEWER_SECRET")]
    secret: Option<String>,
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
            address: self.address,
            port: self.port,
            suppress_output: self
                .suppress_output
                .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")),
            secret: self.secret,
            objectiveai_dir: self.objectiveai_dir,
            objectiveai_state: self.objectiveai_state,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub suppress_output: Option<bool>,
    pub secret: Option<String>,
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
            // Loopback + ephemeral by default: the actual bound port
            // is read back from the listener and published in the
            // viewer lock file, so a fixed default is unnecessary.
            address: self.address.unwrap_or_else(|| "127.0.0.1".to_string()),
            port: self.port.unwrap_or(0),
            suppress_output: self.suppress_output.unwrap_or(false),
            secret: self.secret,
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
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
    pub secret: Option<String>,
    /// Layout root (`OBJECTIVEAI_DIR`); default `~/.objectiveai`.
    pub objectiveai_dir: PathBuf,
    /// State name (`OBJECTIVEAI_STATE`); default `"default"`.
    pub objectiveai_state: String,
}

pub async fn setup(
    config: Config,
) -> std::io::Result<(
    tokio::net::TcpListener,
    axum::Router,
    objectiveai_sdk::viewer::EventSender,
    EventReceiver,
    BinaryExecutor,
    PathBuf,
)> {
    let (tx, rx) = mpsc::unbounded_channel::<Event>();
    let secret = config.secret.map(Arc::new);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.address, config.port)).await?;

    fn built_in_route(
        path: &'static str,
        sub_type: &'static str,
        tx: tokio::sync::mpsc::UnboundedSender<Event>,
    ) -> (&'static str, axum::routing::MethodRouter) {
        let handler = move |Json(value): Json<serde_json::Value>| {
            let tx = tx.clone();
            let sub_type = sub_type.to_string();
            async move {
                let _ = tx.send(Event::Inbound {
                    destination: "objectiveai".to_string(),
                    sub_type,
                    value,
                });
                StatusCode::OK
            }
        };
        (path, axum::routing::post(handler))
    }

    let mut app = axum::Router::new();
    for (path, route) in [
        built_in_route("/agent/completions", "agent_completions", tx.clone()),
        built_in_route("/functions/executions", "functions_executions", tx.clone()),
        built_in_route(
            "/functions/inventions/recursive",
            "functions_inventions_recursive",
            tx.clone(),
        ),
        built_in_route("/laboratories/executions", "laboratories_executions", tx.clone()),
    ] {
        app = app.route(path, route);
    }

    // One executor for everything the viewer runs through the cli
    // binary: plugin discovery here at startup, `cli_run` dispatches
    // from plugin iframes, and `list_plugins_with_viewer` from the
    // shell. OBJECTIVEAI_DIR / OBJECTIVEAI_STATE are stamped onto
    // every spawned child so the cli resolves the same tree the
    // viewer serves `plugin://` assets from, even when the viewer's
    // own config came from a programmatic `ConfigBuilder` rather
    // than the env.
    let executor = BinaryExecutor::new(Some(config.objectiveai_dir.clone()))
        .env(
            "OBJECTIVEAI_DIR",
            config.objectiveai_dir.to_string_lossy().into_owned(),
        )
        .env("OBJECTIVEAI_STATE", config.objectiveai_state.clone());
    let plugins_dir = crate::plugins::plugins_dir(&config.objectiveai_dir);

    // Scan installed plugins and register any viewer routes they
    // declare. Listing is once-at-startup; the user opts in to
    // refresh by restarting the viewer.
    let plugins = crate::plugins::list_all_plugins(&executor).await;
    for plugin in plugins {
        let plugin_name = plugin.name.clone();
        for route in plugin.viewer_routes {
            if !route.path.starts_with('/') {
                eprintln!(
                    "skipping plugin {plugin_name:?} route with non-`/`-prefixed path: {:?}",
                    route.path
                );
                continue;
            }
            app = register_plugin_route(app, tx.clone(), plugin_name.clone(), route);
        }
    }

    let app = app.layer(middleware::from_fn_with_state(secret, signature_middleware));

    // Clone tx for downstream consumers (cli_command Tauri command
    // managed on the Tauri Builder; lets in-process embedders inject
    // synthetic events too).
    let events_tx = tx.clone();
    Ok((listener, app, events_tx, rx, executor, plugins_dir))
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
    listener: tokio::net::TcpListener,
    app: axum::Router,
    events_tx: objectiveai_sdk::viewer::EventSender,
    mut rx: EventReceiver,
    executor: BinaryExecutor,
    plugins_dir: PathBuf,
    exiter_tx: Option<tokio::sync::oneshot::Sender<Exiter>>,
) -> i32 {
    tokio::spawn(async move {
        axum::serve(listener, app).await
    });

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
    let suppress_output = config.suppress_output;
    let lock_dir = config
        .objectiveai_dir
        .join("state")
        .join(&config.objectiveai_state)
        .join("locks");
    let (listener, app, events_tx, rx, executor, plugins_dir) = setup(config).await?;

    // There is only ever ONE viewer per STATE (unlike the api, which
    // is one per OBJECTIVEAI_DIR): claim key "viewer" in
    // <dir>/state/<state>/locks the moment the listen address is
    // known, publishing the URL clients connect with (wildcard binds
    // map to loopback). Anyone can lockfile::try_read it without
    // owning the claim; the claim itself is held until process death
    // (LockClaim leaks on drop by design) and the kernel releases it
    // on any exit, crash included.
    let addr = listener.local_addr()?;
    let connect_ip = match addr.ip() {
        std::net::IpAddr::V4(v4) if v4.is_unspecified() => {
            std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
        }
        std::net::IpAddr::V6(v6) if v6.is_unspecified() => {
            std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
        }
        ip => ip,
    };
    let connect_url =
        format!("http://{}", std::net::SocketAddr::new(connect_ip, addr.port()));
    if objectiveai_sdk::lockfile::try_acquire(&lock_dir, "viewer", &connect_url)
        .await
        .is_none()
    {
        return Err(std::io::Error::other(
            "another objectiveai-viewer instance already holds the viewer lock for this state",
        ));
    }

    if !suppress_output {
        eprintln!("listening on {addr}");
    }
    Ok(serve(listener, app, events_tx, rx, executor, plugins_dir, None))
}
