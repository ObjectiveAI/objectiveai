//! An agent's or a tool's frontend, served to the page's frame from a
//! web server of the app's own on the loopback: every request the frame
//! makes is one daemon frontend request (`agents frontend`,
//! `standalone_tools frontend`, `dependency_tools frontend`), over the page's connection, and its
//! answer comes back as it streams — a WebSocket's bytes both ways
//! included. Nothing passes through the page.
//!
//! The rules:
//!
//! 1. `frontend_open` serves one agent's or tool's frontend on a port of
//!    its own, so every one is an origin of its own, and answers the address
//!    the frame loads: `http://127.0.0.1:<port>/frontend/?__diverge=<key>`.
//! 2. Every request is checked before it is sent on:
//!    a. its `Host` is `127.0.0.1:<port>`, so a name made to point at
//!       the loopback (DNS rebinding) reaches nothing;
//!    b. a request a browser marks as not from the frame's own origin
//!       (`Sec-Fetch-Site` other than `same-origin`) carries the key —
//!       the frame's first load does, from the app's own origin; one
//!       that does not is `403`;
//!    c. a request no browser made (no `Sec-Fetch-Site`) is let through:
//!       a program running as the person can read their identities and
//!       speak to the daemon as them already.
//! 3. The request goes on as it came — method, path and query, headers
//!    — less the key and the hop-by-hop headers (`connection` and
//!    `upgrade` kept on an upgrade); its body is streamed as the daemon
//!    takes it. The answer's head is the response's status and headers,
//!    less the hop-by-hop ones, and its body is streamed as it comes.
//! 4. An upgrade the program answers `101` is a two-way pipe on the
//!    frame's connection: what the frame sends goes to the program, what
//!    the program sends comes back, until either side ends.
//! 5. A request the daemon refuses is a plain page saying why: not
//!    found (`404`), forbidden (`403`), no frontend (`404`), or the
//!    failure (`502`).
//! 6. `frontend_close` ends the frontend: the server stops taking
//!    requests and every request and pipe still open is ended, which
//!    ends its daemon request — so a container the frontend held up is
//!    held no more. The page closes it when its tab goes, and a page
//!    that starts loading closes every one (`close_all`).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use diverge_sdk::daemon::endpoints::{agents, dependency_tools, standalone_tools};
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::containers::frontend::output::Head as Answered;
use diverge_sdk::shared::containers::frontend::request::Head;
use diverge_sdk::tauri_proxy::TauriProxy;
use futures_util::{Stream, StreamExt as _};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager as _, State as Managed};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::{Mutex, watch};

/// The query member the frame's first load carries the key in.
const KEY: &str = "__diverge";

/// The headers that belong to one hop and are never sent on.
const HOP: [&str; 8] = ["connection", "keep-alive", "transfer-encoding", "te", "trailer", "upgrade", "proxy-connection", "proxy-authenticate"];

/// Whose frontend: an agent's, a standalone tool's or a dependency
/// tool's, as the page's `Files` names it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Agent(reference::Agent),
    Tool(reference::Tool),
    Dependency(reference::Dependency),
}

/// A frontend served: its id, and the address the frame loads.
#[derive(Debug, Clone, Serialize)]
pub struct Opened {
    id: u32,
    url: String,
}

/// The frontends being served, by id: each one's word to end.
#[derive(Default)]
pub struct Frontends {
    next: AtomicU32,
    open: Mutex<HashMap<u32, (watch::Sender<bool>, tauri::async_runtime::JoinHandle<()>)>>,
}

impl Frontends {
    /// Rule 6, for every frontend: what a page starting to load does.
    pub async fn close_all(&self) {
        for (_, (ended, serving)) in self.open.lock().await.drain() {
            let _ = ended.send(true);
            serving.abort();
        }
    }
}

/// What every request of one frontend shares.
struct Served {
    app: AppHandle,
    target: Target,
    key: String,
    port: u16,
    /// `true` once the frontend is closed: every request and pipe ends.
    ended: watch::Receiver<bool>,
}

/// A key no one guesses: 128 bits from the process's random hashing seeds.
fn key() -> String {
    use std::hash::{BuildHasher as _, Hasher as _};
    let mut key = String::new();
    for salt in 0..2u8 {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u8(salt);
        hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default());
        key.push_str(&format!("{:016x}", hasher.finish()));
    }
    key
}

/// Rule 1.
#[tauri::command]
pub async fn frontend_open(app: AppHandle, frontends: Managed<'_, Frontends>, target: Target) -> Result<Opened, String> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.map_err(|error| error.to_string())?;
    let port = listener.local_addr().map_err(|error| error.to_string())?.port();
    let key = key();
    let (ended_tx, ended) = watch::channel(false);
    let served = Arc::new(Served { app, target, key: key.clone(), port, ended: ended.clone() });
    let router = Router::new().fallback(forward).with_state(served);
    let mut stopped = ended;
    let serving = tauri::async_runtime::spawn(async move {
        let _ = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = stopped.wait_for(|ended| *ended).await;
            })
            .await;
    });
    let id = frontends.next.fetch_add(1, Ordering::Relaxed);
    frontends.open.lock().await.insert(id, (ended_tx, serving));
    Ok(Opened { id, url: format!("http://127.0.0.1:{port}/frontend/?{KEY}={key}") })
}

/// Rule 6.
#[tauri::command]
pub async fn frontend_close(frontends: Managed<'_, Frontends>, id: u32) -> Result<(), String> {
    if let Some((ended, serving)) = frontends.open.lock().await.remove(&id) {
        let _ = ended.send(true);
        serving.abort();
    }
    Ok(())
}

/// Rule 5: a plain page saying why.
fn page(status: StatusCode, why: &str) -> Response {
    (status, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], why.to_string()).into_response()
}

/// What one answer said, whichever family's it is.
enum Said {
    Head(Answered),
    Body(Bytes),
    NotFound,
    Forbidden,
    NoFrontend,
    Error(String),
}

/// The way into one request, whichever family's it is.
#[derive(Clone)]
enum Writer {
    Agent(agents::frontend::client::execute::Frontend),
    Tool(standalone_tools::frontend::client::execute::Frontend),
    Dependency(dependency_tools::frontend::client::execute::Frontend),
}

impl Writer {
    async fn body(&self, bytes: &[u8]) -> bool {
        match self {
            Writer::Agent(w) => w.body(bytes).await.is_ok(),
            Writer::Tool(w) => w.body(bytes).await.is_ok(),
            Writer::Dependency(w) => w.body(bytes).await.is_ok(),
        }
    }

    async fn end(&self) {
        match self {
            Writer::Agent(w) => {
                let _ = w.end().await;
            }
            Writer::Tool(w) => {
                let _ = w.end().await;
            }
            Writer::Dependency(w) => {
                let _ = w.end().await;
            }
        }
    }
}

/// An answer's frames, as one stream.
type Answers = std::pin::Pin<Box<dyn Stream<Item = Said> + Send>>;

/// Send the request to the daemon: its answers, and the way in.
async fn send(app: &AppHandle, target: &Target, head: Head) -> Result<(Answers, Writer), String> {
    let handle = app.state::<TauriProxy>().link().handle().await.map_err(|refusal| format!("{refusal:?}"))?;
    match target {
        Target::Agent(agent) => {
            use agents::frontend::{client::execute, client::request, server::response::Frame};
            let request = request::Frame { agent: agent.clone(), head, relay: None };
            let (stream, writer) = execute::execute(&handle, &request).await.map_err(|error| error.to_string())?;
            let said = stream.map(|item| match item {
                Ok(Frame::Head(head)) => Said::Head(head),
                Ok(Frame::Body(bytes)) => Said::Body(bytes),
                Ok(Frame::NotFound) => Said::NotFound,
                Ok(Frame::Forbidden) => Said::Forbidden,
                Ok(Frame::NoFrontend) => Said::NoFrontend,
                Ok(Frame::Error(error)) => Said::Error(error.0.to_string()),
                Err(error) => Said::Error(error.to_string()),
            });
            Ok((Box::pin(said), Writer::Agent(writer)))
        }
        Target::Tool(tool) => {
            use standalone_tools::frontend::{client::execute, client::request, server::response::Frame};
            let request = request::Frame { tool: tool.clone(), head, relay: None };
            let (stream, writer) = execute::execute(&handle, &request).await.map_err(|error| error.to_string())?;
            let said = stream.map(|item| match item {
                Ok(Frame::Head(head)) => Said::Head(head),
                Ok(Frame::Body(bytes)) => Said::Body(bytes),
                Ok(Frame::NotFound) => Said::NotFound,
                Ok(Frame::Forbidden) => Said::Forbidden,
                Ok(Frame::NoFrontend) => Said::NoFrontend,
                Ok(Frame::Error(error)) => Said::Error(error.0.to_string()),
                Err(error) => Said::Error(error.to_string()),
            });
            Ok((Box::pin(said), Writer::Tool(writer)))
        }
        Target::Dependency(dependency) => {
            use dependency_tools::frontend::{client::execute, client::request, server::response::Frame};
            let request = request::Frame { dependency: dependency.clone(), head, relay: None };
            let (stream, writer) = execute::execute(&handle, &request).await.map_err(|error| error.to_string())?;
            let said = stream.map(|item| match item {
                Ok(Frame::Head(head)) => Said::Head(head),
                Ok(Frame::Body(bytes)) => Said::Body(bytes),
                Ok(Frame::NotFound) => Said::NotFound,
                Ok(Frame::Forbidden) => Said::Forbidden,
                Ok(Frame::NoFrontend) => Said::NoFrontend,
                Ok(Frame::Error(error)) => Said::Error(error.0.to_string()),
                Err(error) => Said::Error(error.to_string()),
            });
            Ok((Box::pin(said), Writer::Dependency(writer)))
        }
    }
}

/// The path and query as sent on: the key's member taken out.
fn onward(uri: &axum::http::Uri) -> String {
    let path = uri.path();
    match uri.query() {
        None => path.to_string(),
        Some(query) => {
            let kept: Vec<&str> = query.split('&').filter(|member| !(member == &KEY || member.starts_with(&format!("{KEY}=")))).collect();
            if kept.is_empty() { path.to_string() } else { format!("{path}?{}", kept.join("&")) }
        }
    }
}

/// Every request: rules 2 to 5.
async fn forward(State(served): State<Arc<Served>>, mut request: Request) -> Response {
    let headers = request.headers();
    // Rule 2a.
    let host = headers.get(header::HOST).and_then(|value| value.to_str().ok()).unwrap_or_default();
    if host != format!("127.0.0.1:{}", served.port) {
        return page(StatusCode::FORBIDDEN, "this frontend answers its own address only");
    }
    // Rule 2b; 2c is the absence of the header.
    if let Some(site) = headers.get("sec-fetch-site").and_then(|value| value.to_str().ok())
        && site != "same-origin"
    {
        let keyed = request.uri().query().is_some_and(|query| query.split('&').any(|member| member == format!("{KEY}={}", served.key)));
        if !keyed {
            return page(StatusCode::FORBIDDEN, "this frontend is opened from the world's frontend tab");
        }
    }
    // Rule 3.
    let upgrade = headers.contains_key(header::UPGRADE);
    let mut sent = Vec::new();
    for (name, value) in headers {
        let name = name.as_str();
        if name == "host" || (HOP.contains(&name) && !(upgrade && (name == "connection" || name == "upgrade"))) {
            continue;
        }
        if let Ok(value) = value.to_str() {
            sent.push((name.to_string(), value.to_string()));
        }
    }
    let head = Head { method: request.method().to_string(), path: onward(request.uri()), headers: sent };
    // Rule 4: the upgrade, taken before the request is spent.
    let upgrading = upgrade.then(|| hyper::upgrade::on(&mut request));
    let (mut answers, writer) = match send(&served.app, &served.target, head).await {
        Ok(sent) => sent,
        Err(why) => return page(StatusCode::BAD_GATEWAY, &why),
    };
    // Rule 3: the body, streamed beside the answer.
    if !upgrade {
        let writer = writer.clone();
        let mut ended = served.ended.clone();
        let mut body = request.into_body().into_data_stream();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    piece = body.next() => match piece {
                        Some(Ok(bytes)) => {
                            if !writer.body(&bytes).await {
                                return;
                            }
                        }
                        Some(Err(_)) => return,
                        None => break,
                    },
                    () = closed(&mut ended) => return,
                }
            }
            writer.end().await;
        });
    }
    let answered = match answers.next().await {
        Some(Said::Head(head)) => head,
        // Rule 5.
        Some(Said::NotFound) => return page(StatusCode::NOT_FOUND, "the tool was not found, or is not running"),
        Some(Said::Forbidden) => return page(StatusCode::FORBIDDEN, "this account may not open the tool's frontend"),
        Some(Said::NoFrontend) => return page(StatusCode::NOT_FOUND, "the tool's image declares no frontend"),
        Some(Said::Error(why)) => return page(StatusCode::BAD_GATEWAY, &why),
        Some(Said::Body(_)) | None => return page(StatusCode::BAD_GATEWAY, "the tool's frontend did not answer"),
    };
    let status = StatusCode::from_u16(answered.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let switching = status == StatusCode::SWITCHING_PROTOCOLS && upgrade;
    let mut response = Response::builder().status(status);
    for (name, value) in &answered.headers {
        let lower = name.to_ascii_lowercase();
        if HOP.contains(&lower.as_str()) && !(switching && (lower == "connection" || lower == "upgrade")) {
            continue;
        }
        if let (Ok(name), Ok(value)) = (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value)) {
            response = response.header(name, value);
        }
    }
    // Rule 4.
    if switching && let Some(upgrading) = upgrading {
        let ended = served.ended.clone();
        tokio::spawn(pipe(upgrading, answers, writer, ended));
        return response.body(Body::empty()).unwrap_or_else(|_| page(StatusCode::BAD_GATEWAY, "the answer could not be made"));
    }
    // Rule 3: the body, as it comes, ended with the frontend.
    let mut ended = served.ended.clone();
    let body = async_stream(answers, async move { closed(&mut ended).await });
    response.body(Body::from_stream(body)).unwrap_or_else(|_| page(StatusCode::BAD_GATEWAY, "the answer could not be made"))
}

/// The answer's body pieces, until its end, an error, or `ended`.
fn async_stream(answers: Answers, ended: impl std::future::Future<Output = ()> + Send + 'static) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    answers
        .take_until(Box::pin(ended))
        .map(|said| match said {
            Said::Body(bytes) => Ok(bytes),
            Said::Error(why) => Err(std::io::Error::other(why)),
            _ => Ok(Bytes::new()),
        })
}

/// Rule 4: the frame's connection and the tool's, both ways, until
/// either side ends or the frontend does.
async fn pipe(upgrading: hyper::upgrade::OnUpgrade, mut answers: Answers, writer: Writer, mut ended: watch::Receiver<bool>) {
    let Ok(upgraded) = upgrading.await else {
        return;
    };
    let (mut reading, mut writing) = tokio::io::split(TokioIo::new(upgraded));
    let mut buffer = vec![0u8; 64 * 1024];
    let mut open = true;
    loop {
        tokio::select! {
            read = reading.read(&mut buffer), if open => match read {
                Ok(n) if n > 0 => {
                    if !writer.body(&buffer[..n]).await {
                        return;
                    }
                }
                _ => {
                    writer.end().await;
                    open = false;
                }
            },
            said = answers.next() => match said {
                Some(Said::Body(bytes)) => {
                    if writing.write_all(&bytes).await.is_err() {
                        return;
                    }
                }
                _ => {
                    let _ = writing.shutdown().await;
                    return;
                }
            },
            () = closed(&mut ended) => return,
        }
    }
}

/// Resolves once the frontend is closed; a sender gone is closed too.
async fn closed(ended: &mut watch::Receiver<bool>) {
    let _ = ended.wait_for(|ended| *ended).await;
}

/// Rule 6, for the commands: a page starting to load closes them all.
pub async fn page_loading(app: &AppHandle) {
    app.state::<Frontends>().close_all().await;
}
