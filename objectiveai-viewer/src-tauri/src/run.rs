use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use envconfig::Envconfig;
use serde::Serialize;
use subtle::ConstantTimeEq;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{mpsc, Notify};
use crate::agent;
use crate::functions;
use crate::laboratories;

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
}

impl EnvConfigBuilder {
    pub fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            address: self.address,
            port: self.port,
            suppress_output: None,
            secret: self.secret,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub suppress_output: Option<bool>,
    pub secret: Option<String>,
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
            address: self.address.unwrap_or_else(|| "0.0.0.0".to_string()),
            port: self.port.unwrap_or(5001),
            suppress_output: self.suppress_output.unwrap_or(false),
            secret: self.secret,
        }
    }
}

pub struct Config {
    pub address: String,
    pub port: u16,
    pub suppress_output: bool,
    pub secret: Option<String>,
}

pub async fn setup(config: Config) -> std::io::Result<(tokio::net::TcpListener, axum::Router, EventReceiver)> {
    let (tx, rx) = mpsc::unbounded_channel::<Event>();
    let secret = config.secret.map(Arc::new);

    let app = axum::Router::new()
        .route(
            "/agent/completions",
            axum::routing::post({
                let tx = tx.clone();
                move |Json(request): Json<agent::completions::request::Request>| async move {
                    tx.send(Event::AgentCompletions(request)).ok();
                    StatusCode::OK
                }
            }),
        )
        .route(
            "/functions/executions",
            axum::routing::post({
                let tx = tx.clone();
                move |Json(request): Json<functions::executions::request::Request>| async move {
                    tx.send(Event::FunctionsExecutions(request)).ok();
                    StatusCode::OK
                }
            }),
        )
        .route(
            "/functions/inventions/recursive",
            axum::routing::post({
                let tx = tx.clone();
                move |Json(request): Json<functions::inventions::recursive::request::Request>| async move {
                    tx.send(Event::FunctionsInventionsRecursive(request)).ok();
                    StatusCode::OK
                }
            }),
        )
        .route(
            "/laboratories/executions",
            axum::routing::post({
                let tx = tx.clone();
                move |Json(request): Json<laboratories::executions::request::Request>| async move {
                    tx.send(Event::LaboratoriesExecutions(request)).ok();
                    StatusCode::OK
                }
            }),
        )
        .layer(middleware::from_fn_with_state(secret, signature_middleware));

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.address, config.port)).await?;

    Ok((listener, app, rx))
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
    mut rx: EventReceiver,
    exiter_tx: Option<tokio::sync::oneshot::Sender<Exiter>>,
) -> i32 {
    tokio::spawn(async move {
        axum::serve(listener, app).await
    });

    let ready = Arc::new(Notify::new());
    let ready_for_task = ready.clone();

    tauri::Builder::default()
        .manage(ready)
        .invoke_handler(tauri::generate_handler![viewer_ready])
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
                    let _ = handle.emit(event.name(), &event);
                }
                // Forward remaining events directly.
                while let Some(event) = rx.recv().await {
                    let _ = handle.emit(event.name(), &event);
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
    let (listener, app, rx) = setup(config).await?;
    if !suppress_output {
        let addr = listener.local_addr()?;
        eprintln!("listening on {addr}");
    }
    Ok(serve(listener, app, rx, None))
}

async fn signature_middleware(
    State(secret): State<Option<Arc<String>>>,
    request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(secret) = &secret {
        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let headers = &parts.headers;
        let signature = headers
            .get("X-VIEWER-SIGNATURE")
            .or_else(|| headers.get("VIEWER-SIGNATURE"))
            .or_else(|| headers.get("X-OBJECTIVEAI-SIGNATURE"))
            .or_else(|| headers.get("OBJECTIVEAI-SIGNATURE"))
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        if !verify_signature(secret, &bytes, signature) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let rebuilt = axum::http::Request::from_parts(parts, axum::body::Body::from(bytes));
        Ok(next.run(rebuilt).await)
    } else {
        Ok(next.run(request).await)
    }
}

fn verify_signature(secret: &str, _body: &[u8], signature_header: &str) -> bool {
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(hex_sig) else {
        return false;
    };
    // Compute SHA256(secret) and compare against the provided signature.
    // The signature is a static pre-computed value: sha256=<SHA256(secret)>.
    // Knowing the signature does not reveal the secret (preimage resistance).
    use sha2::{Sha256, Digest};
    let expected = Sha256::digest(secret.as_bytes());
    expected.ct_eq(&sig_bytes).into()
}

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum Event {
    AgentCompletions(agent::completions::request::Request),
    FunctionsExecutions(functions::executions::request::Request),
    FunctionsInventionsRecursive(functions::inventions::recursive::request::Request),
    LaboratoriesExecutions(laboratories::executions::request::Request),
}

impl Event {
    fn name(&self) -> &'static str {
        match self {
            Event::AgentCompletions(_) => "agent-completions",
            Event::FunctionsExecutions(_) => "functions-executions",
            Event::FunctionsInventionsRecursive(_) => "functions-inventions-recursive",
            Event::LaboratoriesExecutions(_) => "laboratories-executions",
        }
    }
}

pub type EventReceiver = mpsc::UnboundedReceiver<Event>;
