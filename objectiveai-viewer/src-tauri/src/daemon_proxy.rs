//! The Rust-side daemon proxy: one Tauri command per daemon HTTP
//! endpoint, each dialing the daemon with `reqwest` and streaming the
//! endpoint's raw SSE `data` payloads verbatim into a
//! `tauri::ipc::Channel` — the webview holds NO daemon connections
//! (Chromium's 6-per-origin HTTP/1.1 connection cap starved the
//! direct-fetch model at ~6 concurrent streams). The daemon address,
//! auth signature, and the viewer agent identity live HERE; they
//! never cross into JS.
//!
//! Wire protocol (mirrored by the SDK's `connectViewerStream`):
//! - the command's returned future settles the CONNECT: `Ok(())` once
//!   the daemon's 2xx response headers arrive, `Err(String)` on a
//!   connection failure / non-2xx — the same resolve/reject semantics
//!   as the JS `connectSse`;
//! - stream lifecycle rides the channel as [`StreamEvent`]s: `data`
//!   per SSE payload, `end` on the body ending (the end-of-stream
//!   marker), `error` on a mid-stream transport failure. Channel
//!   messages and the invoke resolution are separate IPC messages
//!   with no ordering guarantee — the JS side attaches `onmessage`
//!   before invoking, so nothing here depends on ordering;
//! - cancellation: the client mints a `stream_id` per stream and
//!   calls [`daemon_stream_close`] to cancel; dropping the in-flight
//!   `reqwest::Response` closes the connection, which cancels the
//!   daemon-side run (the fetch-abort equivalent). A failed
//!   `channel.send` (webview window closed) ends the pump the same
//!   way, so closed windows can't leak daemon streams.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eventsource_stream::Eventsource;
use futures::StreamExt;
use tauri::ipc::Channel;
use tokio_util::sync::CancellationToken;

/// Per-call identity for viewer-initiated daemon runs: instance
/// hierarchy `"Viewer"`, every other field `None` so the daemon
/// clears rather than inherits it — nothing leaks from the daemon's
/// own environment into a viewer-initiated run.
fn viewer_agent_arguments() -> objectiveai_sdk::cli::command::AgentArguments {
    objectiveai_sdk::cli::command::AgentArguments {
        agent_instance_hierarchy: Some("Viewer".to_string()),
        ..Default::default()
    }
}

/// Managed state: the daemon coordinates every proxy dial uses, one
/// pooled client, and the live-stream cancellation registry.
pub struct DaemonProxy {
    /// The daemon's published base address, e.g. `http://127.0.0.1:49152`.
    pub address: String,
    /// The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>` sent as
    /// the `X-OBJECTIVEAI-SIGNATURE` header, or `None` against a
    /// secretless daemon.
    pub signature: Option<String>,
    client: reqwest::Client,
    /// `stream_id → cancellation` for every live stream. Entries are
    /// inserted before the pump spawns and removed by the pump itself
    /// on ANY exit, so [`daemon_stream_close`] on an ended stream is a
    /// no-op.
    streams: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl DaemonProxy {
    pub fn new(address: String, signature: Option<String>) -> Self {
        Self {
            address,
            signature,
            client: reqwest::Client::new(),
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// GET builder for an SSE route: accept header + auth signature.
    fn get(&self, url: String) -> reqwest::RequestBuilder {
        let mut req = self
            .client
            .get(url)
            .header("Accept", "text/event-stream");
        if let Some(signature) = &self.signature {
            req = req.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        req
    }
}

/// One per-stream channel event (serde-tagged; the TS mirror is the
/// SDK's `ViewerStreamEvent`).
#[derive(Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// One SSE event's raw `data` payload, verbatim.
    Data { data: String },
    /// The daemon closed the stream normally (body end).
    End,
    /// Transport error mid-stream; the stream is over.
    Error { message: String },
}

/// A transport error's FULL cause chain — `Display` alone hides the
/// interesting part (reqwest's "error decoding response body" says
/// nothing without the hyper cause underneath).
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

/// Dial `request` and pump its SSE stream into `on_event`. Registers
/// the stream under `stream_id` for [`daemon_stream_close`]; resolves
/// `Ok(())` once the daemon's 2xx response headers arrive and the
/// pump task owns the stream, `Err` on connect failure. The pump
/// always removes its own registry entry when it exits.
async fn proxy_sse(
    streams: Arc<Mutex<HashMap<String, CancellationToken>>>,
    stream_id: String,
    on_event: Channel<StreamEvent>,
    request: reqwest::RequestBuilder,
) -> Result<(), String> {
    let token = CancellationToken::new();
    streams
        .lock()
        .unwrap()
        .insert(stream_id.clone(), token.clone());

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    tokio::spawn({
        let streams = streams.clone();
        let stream_id = stream_id.clone();
        async move {
            let work = async {
                let response = match request.send().await {
                    Ok(response) => response,
                    Err(e) => {
                        let _ = tx.send(Err(format!(
                            "connect daemon sse: {}",
                            error_chain(&e)
                        )));
                        return;
                    }
                };
                if !response.status().is_success() {
                    let _ = tx.send(Err(format!(
                        "connect daemon sse: HTTP {}",
                        response.status()
                    )));
                    return;
                }
                let _ = tx.send(Ok(()));
                let mut events = response.bytes_stream().eventsource();
                while let Some(event) = events.next().await {
                    match event {
                        Ok(event) => {
                            if on_event
                                .send(StreamEvent::Data { data: event.data })
                                .is_err()
                            {
                                // Webview gone — drop the response,
                                // closing the daemon connection.
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = on_event.send(StreamEvent::Error {
                                message: error_chain(&e),
                            });
                            return;
                        }
                    }
                }
                let _ = on_event.send(StreamEvent::End);
            };
            tokio::select! {
                _ = token.cancelled() => {}
                _ = work => {}
            }
            streams.lock().unwrap().remove(&stream_id);
        }
    });

    rx.await
        .unwrap_or_else(|_| Err("stream cancelled before connect completed".to_string()))
}

/// `GET /listen` — the daemon's broadcast SSE.
#[tauri::command]
pub(crate) async fn daemon_listen(
    proxy: tauri::State<'_, DaemonProxy>,
    stream_id: String,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let request = proxy.get(format!("{}/listen", proxy.address));
    proxy_sse(proxy.streams.clone(), stream_id, on_event, request).await
}

/// `POST /execute` — one command run. `request` is the raw
/// `cli::command::Request` JSON, passed through as the body verbatim.
/// Stamps the auth signature and the viewer agent identity — only
/// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` is set (to `"Viewer"`,
/// via [`crate::plugins::viewer_agent_arguments`]); every absent
/// identity header DELETES that config field on the daemon for the
/// run (never inherits).
#[tauri::command]
pub(crate) async fn daemon_execute(
    proxy: tauri::State<'_, DaemonProxy>,
    stream_id: String,
    request: String,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let mut req = proxy
        .client
        .post(format!("{}/execute", proxy.address))
        .header("Accept", "text/event-stream")
        .header("Content-Type", "application/json")
        .body(request);
    if let Some(signature) = &proxy.signature {
        req = req.header("X-OBJECTIVEAI-SIGNATURE", signature);
    }
    let args = viewer_agent_arguments();
    if let Some(v) = &args.agent_instance_hierarchy {
        req = req.header("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY", v);
    }
    if let Some(v) = &args.agent_id {
        req = req.header("X-OBJECTIVEAI-AGENT-ID", v);
    }
    if let Some(v) = &args.agent_full_id {
        req = req.header("X-OBJECTIVEAI-AGENT-FULL-ID", v);
    }
    if let Some(v) = &args.agent_remote {
        req = req.header("X-OBJECTIVEAI-AGENT-REMOTE", v);
    }
    if let Some(v) = &args.response_id {
        req = req.header("X-OBJECTIVEAI-RESPONSE-ID", v);
    }
    if let Some(v) = &args.response_ids {
        req = req.header("X-OBJECTIVEAI-RESPONSE-IDS", v);
    }
    proxy_sse(proxy.streams.clone(), stream_id, on_event, req).await
}

/// `GET /agents/instances/list` — the live all-agents view.
#[tauri::command]
pub(crate) async fn daemon_agents_instances_list(
    proxy: tauri::State<'_, DaemonProxy>,
    stream_id: String,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let request = proxy.get(format!("{}/agents/instances/list", proxy.address));
    proxy_sse(proxy.streams.clone(), stream_id, on_event, request).await
}

/// `GET /agents/instances/{aih}` — one agent's conversation +
/// status. `aih` is the raw agent instance hierarchy (contains `/`),
/// concatenated exactly like the fetch-mode URL template.
#[tauri::command]
pub(crate) async fn daemon_agents_instance(
    proxy: tauri::State<'_, DaemonProxy>,
    stream_id: String,
    aih: String,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let request = proxy.get(format!("{}/agents/instances/{}", proxy.address, aih));
    proxy_sse(proxy.streams.clone(), stream_id, on_event, request).await
}

/// `GET /laboratories/list` — the live laboratories view.
#[tauri::command]
pub(crate) async fn daemon_laboratories_list(
    proxy: tauri::State<'_, DaemonProxy>,
    stream_id: String,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let request = proxy.get(format!("{}/laboratories/list", proxy.address));
    proxy_sse(proxy.streams.clone(), stream_id, on_event, request).await
}

/// `GET /laboratories/{id}` — one laboratory's live record, with the
/// optional `(machine, machine_state)` host pin as query params (the
/// daemon treats a partial pair as unpinned).
#[tauri::command]
pub(crate) async fn daemon_laboratory(
    proxy: tauri::State<'_, DaemonProxy>,
    stream_id: String,
    id: String,
    machine: Option<String>,
    machine_state: Option<String>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let mut request = proxy.get(format!("{}/laboratories/{}", proxy.address, id));
    if let Some(machine) = machine {
        request = request.query(&[("machine", machine)]);
    }
    if let Some(machine_state) = machine_state {
        request = request.query(&[("machine_state", machine_state)]);
    }
    proxy_sse(proxy.streams.clone(), stream_id, on_event, request).await
}

/// `GET /laboratories/{id}/filetree` — one laboratory's live file
/// tree, with the same optional host pin as [`daemon_laboratory`].
#[tauri::command]
pub(crate) async fn daemon_laboratory_filetree(
    proxy: tauri::State<'_, DaemonProxy>,
    stream_id: String,
    id: String,
    machine: Option<String>,
    machine_state: Option<String>,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let mut request = proxy.get(format!("{}/laboratories/{}/filetree", proxy.address, id));
    if let Some(machine) = machine {
        request = request.query(&[("machine", machine)]);
    }
    if let Some(machine_state) = machine_state {
        request = request.query(&[("machine_state", machine_state)]);
    }
    proxy_sse(proxy.streams.clone(), stream_id, on_event, request).await
}

/// `GET /user` — the user-requests channel: every pending request
/// replayed on connect (then the `live` caught-up marker), live
/// requests / settlements / timeouts after.
#[tauri::command]
pub(crate) async fn daemon_user(
    proxy: tauri::State<'_, DaemonProxy>,
    stream_id: String,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let request = proxy.get(format!("{}/user", proxy.address));
    proxy_sse(proxy.streams.clone(), stream_id, on_event, request).await
}

/// `POST /user/{id}/reply` — answer one pending user request as the
/// VIEWER. Stamps the auth signature and the viewer agent identity
/// (the same header set [`daemon_execute`] stamps — the replier
/// identity the daemon reports to the originating command). Unlike
/// the streaming commands this returns a VALUE: the daemon's
/// `UserReplyOutcome` JSON (`accepted` / `rejected` / `settled` /
/// `not_found`), whatever the HTTP status — only transport and parse
/// failures are `Err`.
#[tauri::command]
pub(crate) async fn daemon_user_reply(
    proxy: tauri::State<'_, DaemonProxy>,
    id: String,
    reply: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut req = proxy
        .client
        .post(format!("{}/user/{}/reply", proxy.address, id))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "reply": reply }));
    if let Some(signature) = &proxy.signature {
        req = req.header("X-OBJECTIVEAI-SIGNATURE", signature);
    }
    let args = viewer_agent_arguments();
    if let Some(v) = &args.agent_instance_hierarchy {
        req = req.header("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY", v);
    }
    if let Some(v) = &args.agent_id {
        req = req.header("X-OBJECTIVEAI-AGENT-ID", v);
    }
    if let Some(v) = &args.agent_full_id {
        req = req.header("X-OBJECTIVEAI-AGENT-FULL-ID", v);
    }
    if let Some(v) = &args.agent_remote {
        req = req.header("X-OBJECTIVEAI-AGENT-REMOTE", v);
    }
    if let Some(v) = &args.response_id {
        req = req.header("X-OBJECTIVEAI-RESPONSE-ID", v);
    }
    if let Some(v) = &args.response_ids {
        req = req.header("X-OBJECTIVEAI-RESPONSE-IDS", v);
    }
    let response = req
        .send()
        .await
        .map_err(|e| format!("user reply: {}", error_chain(&e)))?;
    let body = response
        .text()
        .await
        .map_err(|e| format!("user reply body: {}", error_chain(&e)))?;
    serde_json::from_str(&body)
        .map_err(|e| format!("user reply outcome parse: {e}: {body}"))
}

/// Cancel a live proxy stream by its client-minted id. No-op on an
/// unknown id (the stream already ended and cleaned itself up), so
/// clients may call it unconditionally on teardown.
#[tauri::command]
pub(crate) fn daemon_stream_close(proxy: tauri::State<'_, DaemonProxy>, stream_id: String) {
    if let Some(token) = proxy.streams.lock().unwrap().remove(&stream_id) {
        token.cancel();
    }
}
