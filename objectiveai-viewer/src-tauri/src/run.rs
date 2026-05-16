use axum::Json;
use axum::http::StatusCode;
use axum::middleware;
use envconfig::Envconfig;
use objectiveai_sdk::HttpClient;
use objectiveai_sdk::agent::completions::request::AgentCompletionNotifyParams;
use objectiveai_sdk::filesystem::Client as FsClient;
use objectiveai_sdk::filesystem::plugins::ManifestWithNameAndSource;
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

#[tauri::command]
async fn notify_agent_completion(
    state: tauri::State<'_, HttpClient>,
    params: AgentCompletionNotifyParams,
) -> Result<(), String> {
    objectiveai_sdk::agent::completions::notify_agent_completion(
        state.inner(),
        params,
    )
    .await
    .map_err(|e| e.to_string())
}

#[derive(Envconfig)]
struct EnvConfigBuilder {
    // -- HttpClient fields (identical order across all 3 structs) --
    #[envconfig(from = "OBJECTIVEAI_ADDRESS")]
    objectiveai_address: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_AUTHORIZATION")]
    objectiveai_authorization: Option<String>,
    #[envconfig(from = "OPENROUTER_ADDRESS")]
    openrouter_address: Option<String>,
    #[envconfig(from = "OPENROUTER_AUTHORIZATION")]
    openrouter_authorization: Option<String>,
    #[envconfig(from = "GITHUB_AUTHORIZATION")]
    github_authorization: Option<String>,
    #[envconfig(from = "MCP_AUTHORIZATION")]
    mcp_authorization: Option<String>,
    #[envconfig(from = "VIEWER_SIGNATURE")]
    viewer_signature: Option<String>,
    #[envconfig(from = "USER_AGENT")]
    user_agent: Option<String>,
    #[envconfig(from = "HTTP_REFERER")]
    http_referer: Option<String>,
    #[envconfig(from = "X_TITLE")]
    x_title: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_NAME")]
    commit_author_name: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_EMAIL")]
    commit_author_email: Option<String>,
    // -- Other fields --
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "VIEWER_SECRET")]
    secret: Option<String>,
    #[envconfig(from = "CONFIG_BASE_DIR")]
    config_base_dir: Option<String>,
}

impl EnvConfigBuilder {
    pub fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            // -- HttpClient fields --
            objectiveai_address: self.objectiveai_address,
            objectiveai_authorization: self.objectiveai_authorization,
            openrouter_address: self.openrouter_address,
            openrouter_authorization: self.openrouter_authorization,
            github_authorization: self.github_authorization,
            mcp_authorization: self.mcp_authorization,
            viewer_signature: self.viewer_signature,
            user_agent: self.user_agent,
            http_referer: self.http_referer,
            x_title: self.x_title,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            // -- Other fields --
            address: self.address,
            port: self.port,
            suppress_output: None,
            secret: self.secret,
            config_base_dir: self.config_base_dir,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    // -- HttpClient fields (identical order across all 3 structs) --
    pub objectiveai_address: Option<String>,
    pub objectiveai_authorization: Option<String>,
    pub openrouter_address: Option<String>,
    pub openrouter_authorization: Option<String>,
    pub github_authorization: Option<String>,
    pub mcp_authorization: Option<String>,
    pub viewer_signature: Option<String>,
    pub user_agent: Option<String>,
    pub http_referer: Option<String>,
    pub x_title: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    // -- Other fields --
    pub address: Option<String>,
    pub port: Option<u16>,
    pub suppress_output: Option<bool>,
    pub secret: Option<String>,
    pub config_base_dir: Option<String>,
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
            // -- HttpClient fields --
            objectiveai_address: self.objectiveai_address,
            objectiveai_authorization: self.objectiveai_authorization,
            openrouter_address: self.openrouter_address,
            openrouter_authorization: self.openrouter_authorization,
            github_authorization: self.github_authorization,
            mcp_authorization: self.mcp_authorization,
            viewer_signature: self.viewer_signature,
            user_agent: self.user_agent,
            http_referer: self.http_referer,
            x_title: self.x_title,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            // -- Other fields --
            address: self.address.unwrap_or_else(|| "0.0.0.0".to_string()),
            port: self.port.unwrap_or(5001),
            suppress_output: self.suppress_output.unwrap_or(false),
            secret: self.secret,
            config_base_dir: self.config_base_dir,
        }
    }
}

pub struct Config {
    // -- HttpClient fields (identical order across all 3 structs) --
    pub objectiveai_address: Option<String>,
    pub objectiveai_authorization: Option<String>,
    pub openrouter_address: Option<String>,
    pub openrouter_authorization: Option<String>,
    pub github_authorization: Option<String>,
    pub mcp_authorization: Option<String>,
    pub viewer_signature: Option<String>,
    pub user_agent: Option<String>,
    pub http_referer: Option<String>,
    pub x_title: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    // -- Other fields --
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
    pub secret: Option<String>,
    pub config_base_dir: Option<String>,
}

pub async fn setup(config: Config) -> std::io::Result<(tokio::net::TcpListener, axum::Router, objectiveai_sdk::viewer::EventSender, EventReceiver, HttpClient, FsClient)> {
    let (tx, rx) = mpsc::unbounded_channel::<Event>();
    let secret = config.secret.map(Arc::new);

    let mcp_authorization: Option<std::collections::HashMap<String, String>> =
        config.mcp_authorization.and_then(|s| serde_json::from_str(&s).ok());

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.address, config.port)).await?;
    let viewer_address = format!("http://{}", listener.local_addr()?);

    let commit_author_name = config.commit_author_name.clone();
    let commit_author_email = config.commit_author_email.clone();
    let http_client = HttpClient::new(
        reqwest::Client::new(),
        config.objectiveai_address,
        config.objectiveai_authorization,
        config.user_agent,
        config.x_title,
        config.http_referer,
        config.github_authorization,
        config.openrouter_authorization,
        mcp_authorization,
        config.viewer_signature,
        Some(viewer_address),
        config.commit_author_name,
        config.commit_author_email,
    );

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

    let fs_client = FsClient::new(
        config.config_base_dir.as_deref(),
        commit_author_name.as_deref(),
        commit_author_email.as_deref(),
    );

    // Scan installed plugins and register any viewer routes they
    // declare. Listing is once-at-startup; the user opts in to
    // refresh by restarting the viewer.
    let plugins: Vec<ManifestWithNameAndSource> = fs_client.list_plugins(0, usize::MAX).await;
    for plugin in plugins {
        let plugin_name = plugin.name.clone();
        for route in plugin.manifest.viewer_routes {
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
    Ok((listener, app, events_tx, rx, http_client, fs_client))
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
    http_client: HttpClient,
    fs_client: FsClient,
    exiter_tx: Option<tokio::sync::oneshot::Sender<Exiter>>,
) -> i32 {
    tokio::spawn(async move {
        axum::serve(listener, app).await
    });

    let ready = Arc::new(Notify::new());
    let ready_for_task = ready.clone();

    let plugins_dir_for_protocol = fs_client.plugins_dir();
    let builder = tauri::Builder::default()
        .manage(ready)
        .manage(http_client)
        .manage(fs_client)
        .manage(events_tx)
        .register_uri_scheme_protocol("plugin", move |_app, request| {
            serve_plugin_asset(&plugins_dir_for_protocol, request)
        });
    #[cfg(feature = "cli")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        viewer_ready,
        notify_agent_completion,
        crate::plugins::list_plugins_with_viewer,
        crate::cli_command::cli_run,
        crate::api_call::api_call_run,
    ]);
    #[cfg(not(feature = "cli"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        viewer_ready,
        notify_agent_completion,
        crate::plugins::list_plugins_with_viewer,
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
    let (listener, app, events_tx, rx, http_client, fs_client) = setup(config).await?;
    if !suppress_output {
        let addr = listener.local_addr()?;
        eprintln!("listening on {addr}");
    }
    Ok(serve(listener, app, events_tx, rx, http_client, fs_client, None))
}
