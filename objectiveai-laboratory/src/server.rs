//! The per-laboratory request server — a mini-conduit for ONE
//! laboratory.
//!
//! Every laboratory-scoped [`ChannelRequest`] arriving over a daemon
//! `/laboratory` WS lands here, after the
//! [`HostServer`](crate::host::HostServer) demuxes it by
//! `laboratory_id`. MCP ops run against the container's MCP server on its
//! published loopback port through per-response-id
//! [`objectiveai_sdk::mcp::Connection`]s (the same registry shape the
//! CLI conduit keeps for plugins, scoped to one laboratory); the
//! transfer ops park `/export` streams and `/import` bodies exactly
//! like the conduit used to before this logic moved here.

use std::sync::Arc;

use dashmap::DashMap;
use indexmap::IndexMap;
use objectiveai_sdk::laboratories::daemon::{
    ChannelRequest, ChannelResponse, DropResult, ExportBeginRequest, ExportChunk,
    ImportBeginRequest, ImportEndResult, ImportWriteRequest, InitializeReply,
    JsonRpcResult, LocalTransferResult, RequestPayload, ResponsePayload, TransferAck,
    TransferBeginResult, TransferIdRequest,
};
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequestParams, ReadResourceResult,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, ListToolsResult,
};

/// Raw bytes per `LaboratoryExportRead` chunk (base64 on the wire).
const TRANSFER_CHUNK_SIZE: usize = 2 * 1024 * 1024;

/// A transfer half untouched this long was abandoned by its driver —
/// swept lazily on every Begin.
const TRANSFER_IDLE_SECS: i64 = 300;

/// One parked laboratory-transfer half (see the conduit's former
/// implementation — moved here verbatim in shape).
enum TransferEntry {
    Export {
        response: tokio::sync::Mutex<Option<reqwest::Response>>,
        last_used: std::sync::atomic::AtomicI64,
    },
    Import {
        tx: tokio::sync::Mutex<Option<tokio::sync::mpsc::Sender<Result<Vec<u8>, std::io::Error>>>>,
        bytes: std::sync::atomic::AtomicU64,
        join: tokio::sync::Mutex<Option<tokio::task::JoinHandle<Result<(), String>>>>,
        last_used: std::sync::atomic::AtomicI64,
    },
}

impl TransferEntry {
    fn touch(&self) {
        let (TransferEntry::Export { last_used, .. }
        | TransferEntry::Import { last_used, .. }) = self;
        last_used.store(now_secs(), std::sync::atomic::Ordering::Relaxed);
    }

    fn idle_secs(&self) -> i64 {
        let (TransferEntry::Export { last_used, .. }
        | TransferEntry::Import { last_used, .. }) = self;
        now_secs() - last_used.load(std::sync::atomic::Ordering::Relaxed)
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The one-laboratory server: MCP session registry + transfer registry
/// + the container's loopback base URL.
pub struct LabServer {
    /// The container's MCP/transfer HTTP base (`http://127.0.0.1:{port}`).
    base_url: String,
    mcp: objectiveai_sdk::mcp::Client,
    /// Per-response-id MCP connections into the container.
    connections: DashMap<String, Arc<objectiveai_sdk::mcp::Connection>>,
    /// Parked transfer halves, keyed by manager-minted transfer id.
    transfers: DashMap<String, Arc<TransferEntry>>,
}

impl LabServer {
    pub fn new(base_url: String) -> Self {
        use std::time::Duration;
        // Match the CLI conduit's MCP client knobs (100ms/0.5/1.5/1s
        // backoff, 10-minute budget + call timeout — the laboratory is
        // loopback, so these are generous).
        let mcp = objectiveai_sdk::mcp::Client::new(
            reqwest::Client::new(),
            "objectiveai-laboratory".to_string(),
            String::new(),
            String::new(),
            Some(Duration::from_secs(600)),
            Duration::from_millis(100),
            Duration::from_millis(100),
            0.5,
            1.5,
            Duration::from_millis(1000),
            Duration::from_secs(600),
            Some(Duration::from_secs(600)),
        );
        Self {
            base_url,
            mcp,
            connections: DashMap::new(),
            transfers: DashMap::new(),
        }
    }

    /// Serve one request; the reply echoes the correlation id. The
    /// host has already demuxed on `laboratory_id` — this server IS
    /// that laboratory's.
    pub async fn handle(self: &Arc<Self>, request: ChannelRequest) -> ChannelResponse {
        let ChannelRequest { id, headers, payload, .. } = request;
        let payload = match payload {
            RequestPayload::Initialize => self.initialize(&headers).await,
            RequestPayload::SessionTerminate => self.session_terminate(&headers).await,
            RequestPayload::ToolsList(params) => ResponsePayload::ToolsList(
                self.call::<ListToolsRequest, ListToolsResult>(&headers, "tools/list", &params)
                    .await,
            ),
            RequestPayload::ToolsCall(params) => ResponsePayload::ToolsCall(
                self.call::<CallToolRequestParams, CallToolResult>(
                    &headers,
                    "tools/call",
                    &params,
                )
                .await,
            ),
            RequestPayload::ResourcesList(params) => ResponsePayload::ResourcesList(
                self.call::<ListResourcesRequest, ListResourcesResult>(
                    &headers,
                    "resources/list",
                    &params,
                )
                .await,
            ),
            RequestPayload::ResourcesRead(params) => ResponsePayload::ResourcesRead(
                self.call::<ReadResourceRequestParams, ReadResourceResult>(
                    &headers,
                    "resources/read",
                    &params,
                )
                .await,
            ),
            RequestPayload::Drop(req) => {
                // Drop = kill for this response id's session, no upstream
                // DELETE (mirrors the conduit's Drop semantics).
                let dropped = self.connections.remove(&req.response_id).is_some();
                ResponsePayload::Drop(DropResult { dropped })
            }
            RequestPayload::ExportBegin(req) => self.export_begin(req).await,
            RequestPayload::ExportRead(req) => self.export_read(req).await,
            RequestPayload::ExportAbort(req) => self.export_abort(req),
            RequestPayload::ImportBegin(req) => self.import_begin(req).await,
            RequestPayload::ImportWrite(req) => self.import_write(req).await,
            RequestPayload::ImportEnd(req) => self.import_end(req).await,
            RequestPayload::ImportAbort(req) => self.import_abort(req),
            // Host-level ops — answered by the HostServer BEFORE the
            // per-lab demux; reaching here is a routing bug, but the
            // reply shape still pairs correctly.
            RequestPayload::Create(_) => ResponsePayload::Create(rpc_err(
                -32601,
                "laboratory server does not serve create (host-level op)".into(),
            )),
            RequestPayload::Delete(_) => ResponsePayload::Delete(rpc_err(
                -32601,
                "laboratory server does not serve delete (host-level op)".into(),
            )),
            RequestPayload::LocalTransfer(_) => ResponsePayload::LocalTransfer(rpc_err(
                -32601,
                "laboratory server does not serve local transfer (host-level op)".into(),
            )),
        };
        ChannelResponse { id, payload }
    }

    // ── MCP session ops ──────────────────────────────────────────

    async fn initialize(
        &self,
        headers: &IndexMap<String, String>,
    ) -> ResponsePayload {
        let initialize_err = |code: i64, message: String| {
            ResponsePayload::Initialize(JsonRpcResult::Err { code, message, data: None })
        };
        let Some(response_id) = response_id_from_headers(headers) else {
            return initialize_err(-32600, "missing X-OBJECTIVEAI-RESPONSE-ID header".into());
        };
        let connect_headers = sanitize_connect_headers(headers);
        let connection = match self
            .mcp
            .connect(format!("{}/", self.base_url), None, Some(connect_headers))
            .await
        {
            Ok(c) => c,
            Err(e) => return initialize_err(-32603, format!("connect: {e}")),
        };
        let mcp_session_id = connection.session_id.clone();
        let result = connection.initialize_result.clone();
        self.connections.insert(response_id, Arc::new(connection));
        ResponsePayload::Initialize(JsonRpcResult::Ok {
            result: InitializeReply {
                mcp_session_id,
                result,
            },
        })
    }

    async fn session_terminate(
        &self,
        headers: &IndexMap<String, String>,
    ) -> ResponsePayload {
        let ok = || ResponsePayload::SessionTerminate(JsonRpcResult::Ok { result: () });
        let Some(response_id) = response_id_from_headers(headers) else {
            return ok();
        };
        let Some(connection) = self.connections.get(&response_id).map(|c| Arc::clone(&c)) else {
            return ok();
        };
        match connection.delete().await {
            Ok(()) => {
                self.connections.remove(&response_id);
                ok()
            }
            Err(e) => ResponsePayload::SessionTerminate(JsonRpcResult::Err {
                code: -32603,
                message: format!("laboratory: upstream delete: {e}"),
                data: None,
            }),
        }
    }

    /// Raw JSON-RPC POST through the response id's connection —
    /// the conduit's `upstream_call`, scoped to this laboratory.
    async fn call<P, R>(
        &self,
        headers: &IndexMap<String, String>,
        method: &str,
        params: &P,
    ) -> JsonRpcResult<R>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let Some(response_id) = response_id_from_headers(headers) else {
            return rpc_err(-32600, "missing X-OBJECTIVEAI-RESPONSE-ID header".into());
        };
        let Some(conn) = self.connections.get(&response_id).map(|c| Arc::clone(&c)) else {
            return rpc_err(
                -32001,
                format!("no cached connection for response id {response_id:?}"),
            );
        };
        match raw_call(&conn, headers, method, params).await {
            Ok(result) => result,
            Err(message) => rpc_err(-32603, format!("laboratory: {message}")),
        }
    }

    // ── Transfer ops (moved verbatim in shape from the conduit) ──

    fn gc_transfers(&self) {
        self.transfers
            .retain(|_, entry| entry.idle_secs() < TRANSFER_IDLE_SECS);
    }

    /// The local-transfer fast path: pipe this laboratory's `/export`
    /// stream STRAIGHT into `destination`'s `/import` body — no chunk
    /// staging, no base64, no parked transfer entries; reqwest streams
    /// the tar end to end and the HTTP bodies provide the
    /// backpressure. Returns the byte total the destination ingested.
    /// Dropping the export response on any failure aborts the GET, so
    /// nothing leaks.
    pub async fn pipe_export_into(
        &self,
        source_path: &str,
        destination: &LabServer,
        destination_path: &str,
    ) -> Result<u64, String> {
        let export = reqwest::Client::new()
            .get(format!("{}/export", self.base_url))
            .query(&[("path", &source_path)])
            .send()
            .await
            .map_err(|e| format!("export: {e}"))?;
        if !export.status().is_success() {
            let status = export.status();
            let body = export.text().await.unwrap_or_default();
            return Err(format!("export: HTTP {status}: {}", body.trim()));
        }
        // Count bytes as they flow — the import side reports only
        // HTTP success, and the export stream is consumed exactly
        // once, here.
        let bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counted = {
            use futures::StreamExt as _;
            let bytes = std::sync::Arc::clone(&bytes);
            export.bytes_stream().map(move |chunk| {
                if let Ok(chunk) = &chunk {
                    bytes.fetch_add(
                        chunk.len() as u64,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
                chunk
            })
        };
        let import = reqwest::Client::new()
            .post(format!("{}/import", destination.base_url))
            .query(&[("path", &destination_path)])
            .body(reqwest::Body::wrap_stream(counted))
            .send()
            .await
            .map_err(|e| format!("import: {e}"))?;
        if !import.status().is_success() {
            let status = import.status();
            let body = import.text().await.unwrap_or_default();
            return Err(format!("import: HTTP {status}: {}", body.trim()));
        }
        Ok(bytes.load(std::sync::atomic::Ordering::Relaxed))
    }

    async fn export_begin(
        &self,
        req: ExportBeginRequest,
    ) -> ResponsePayload {
                let err = |m: String| ResponsePayload::ExportBegin(rpc_err(-32603, m));
        self.gc_transfers();
        let response = match reqwest::Client::new()
            .get(format!("{}/export", self.base_url))
            .query(&[("path", &req.path)])
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return err(format!("export: {e}")),
        };
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return err(format!("export: HTTP {status}: {}", body.trim()));
        }
        let transfer_id = uuid::Uuid::new_v4().to_string();
        let entry = TransferEntry::Export {
            response: tokio::sync::Mutex::new(Some(response)),
            last_used: std::sync::atomic::AtomicI64::new(now_secs()),
        };
        self.transfers.insert(transfer_id.clone(), Arc::new(entry));
        ResponsePayload::ExportBegin(JsonRpcResult::Ok {
            result: TransferBeginResult { transfer_id },
        })
    }

    async fn export_read(
        &self,
        req: TransferIdRequest,
    ) -> ResponsePayload {
        use base64::Engine as _;
                let err = |m: String| ResponsePayload::ExportRead(rpc_err(-32603, m));
        let entry = match self.transfers.get(&req.transfer_id) {
            Some(e) => Arc::clone(&e),
            None => return err(format!("no export transfer '{}'", req.transfer_id)),
        };
        let TransferEntry::Export { response, .. } = &*entry else {
            return err(format!("transfer '{}' is an import", req.transfer_id));
        };
        entry.touch();
        let mut guard = response.lock().await;
        let Some(live) = guard.as_mut() else {
            return err(format!("export transfer '{}' already closed", req.transfer_id));
        };
        let mut buf: Vec<u8> = Vec::new();
        let mut eof = false;
        while buf.len() < TRANSFER_CHUNK_SIZE {
            match live.chunk().await {
                Ok(Some(bytes)) => buf.extend_from_slice(&bytes),
                Ok(None) => {
                    eof = true;
                    break;
                }
                Err(e) => {
                    *guard = None;
                    drop(guard);
                    self.transfers.remove(&req.transfer_id);
                    return err(format!("export stream: {e}"));
                }
            }
        }
        if eof {
            *guard = None;
            drop(guard);
            self.transfers.remove(&req.transfer_id);
        }
        ResponsePayload::ExportRead(JsonRpcResult::Ok {
            result: ExportChunk {
                data: base64::engine::general_purpose::STANDARD.encode(&buf),
                eof,
            },
        })
    }

    fn export_abort(
        &self,
        req: TransferIdRequest,
    ) -> ResponsePayload {
        self.transfers.remove(&req.transfer_id);
        ResponsePayload::ExportAbort(JsonRpcResult::Ok {
            result: TransferAck {},
        })
    }

    async fn import_begin(
        &self,
        req: ImportBeginRequest,
    ) -> ResponsePayload {
                let err = |m: String| ResponsePayload::ImportBegin(rpc_err(-32603, m));
        self.gc_transfers();
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(4);
        let base = self.base_url.clone();
                let path = req.path.clone();
        let join = tokio::spawn(async move {
            let response = reqwest::Client::new()
                .post(format!("{base}/import"))
                .query(&[("path", &path)])
                .body(reqwest::Body::wrap_stream(
                    tokio_stream::wrappers::ReceiverStream::new(rx),
                ))
                .send()
                .await
                .map_err(|e| format!("import: {e}"))?;
            if response.status().is_success() {
                Ok(())
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                Err(format!("import: HTTP {status}: {}", body.trim()))
            }
        });
        let transfer_id = uuid::Uuid::new_v4().to_string();
        let entry = TransferEntry::Import {
            tx: tokio::sync::Mutex::new(Some(tx)),
            bytes: std::sync::atomic::AtomicU64::new(0),
            join: tokio::sync::Mutex::new(Some(join)),
            last_used: std::sync::atomic::AtomicI64::new(now_secs()),
        };
        self.transfers.insert(transfer_id.clone(), Arc::new(entry));
        ResponsePayload::ImportBegin(JsonRpcResult::Ok {
            result: TransferBeginResult { transfer_id },
        })
    }

    async fn import_write(
        &self,
        req: ImportWriteRequest,
    ) -> ResponsePayload {
        use base64::Engine as _;
                let err = |m: String| ResponsePayload::ImportWrite(rpc_err(-32603, m));
        let entry = match self.transfers.get(&req.transfer_id) {
            Some(e) => Arc::clone(&e),
            None => return err(format!("no import transfer '{}'", req.transfer_id)),
        };
        let TransferEntry::Import { tx, bytes, join, .. } = &*entry else {
            return err(format!("transfer '{}' is an export", req.transfer_id));
        };
        entry.touch();
        let data = match base64::engine::general_purpose::STANDARD.decode(&req.data) {
            Ok(d) => d,
            Err(e) => return err(format!("chunk is not valid base64: {e}")),
        };
        let guard = tx.lock().await;
        let Some(sender) = guard.as_ref() else {
            return err(format!("import transfer '{}' already closed", req.transfer_id));
        };
        let len = data.len() as u64;
        if sender.send(Ok(data)).await.is_err() {
            drop(guard);
            let joined = join.lock().await.take();
            self.transfers.remove(&req.transfer_id);
            let detail = match joined {
                Some(handle) => match handle.await {
                    Ok(Ok(())) => "import ended early".to_string(),
                    Ok(Err(m)) => m,
                    Err(e) => format!("import task panicked: {e}"),
                },
                None => "import body closed".to_string(),
            };
            return err(detail);
        }
        bytes.fetch_add(len, std::sync::atomic::Ordering::Relaxed);
        ResponsePayload::ImportWrite(JsonRpcResult::Ok {
            result: TransferAck {},
        })
    }

    async fn import_end(
        &self,
        req: TransferIdRequest,
    ) -> ResponsePayload {
                let err = |m: String| ResponsePayload::ImportEnd(rpc_err(-32603, m));
        let entry = match self.transfers.remove(&req.transfer_id) {
            Some((_, e)) => e,
            None => return err(format!("no import transfer '{}'", req.transfer_id)),
        };
        let TransferEntry::Import { tx, bytes, join, .. } = &*entry else {
            return err(format!("transfer '{}' is an export", req.transfer_id));
        };
        tx.lock().await.take();
        let joined = join.lock().await.take();
        match joined {
            Some(handle) => match handle.await {
                Ok(Ok(())) => ResponsePayload::ImportEnd(JsonRpcResult::Ok {
                    result: ImportEndResult {
                        bytes: bytes.load(std::sync::atomic::Ordering::Relaxed),
                    },
                }),
                Ok(Err(m)) => err(m),
                Err(e) => err(format!("import task panicked: {e}")),
            },
            None => err(format!("import transfer '{}' already ended", req.transfer_id)),
        }
    }

    fn import_abort(
        &self,
        req: TransferIdRequest,
    ) -> ResponsePayload {
        if let Some((_, entry)) = self.transfers.remove(&req.transfer_id) {
            if let TransferEntry::Import { join, .. } = &*entry {
                if let Ok(mut guard) = join.try_lock() {
                    guard.take();
                }
            }
        }
        ResponsePayload::ImportAbort(JsonRpcResult::Ok {
            result: TransferAck {},
        })
    }
}

fn rpc_err<T>(code: i64, message: String) -> JsonRpcResult<T> {
    JsonRpcResult::Err { code, message, data: None }
}

fn response_id_from_headers(headers: &IndexMap<String, String>) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-ID"))
        .map(|(_, v)| v.clone())
}

fn sanitize_connect_headers(headers: &IndexMap<String, String>) -> IndexMap<String, String> {
    let mut out = headers.clone();
    for k in [
        "Host",
        "host",
        "Content-Length",
        "content-length",
        "Mcp-Session-Id",
        "mcp-session-id",
    ] {
        out.shift_remove(k);
    }
    out
}

/// Raw JSON-RPC POST through an `mcp::Connection` — the conduit's
/// `upstream_call`, with a plain-`String` error.
async fn raw_call<P, R>(
    conn: &objectiveai_sdk::mcp::Connection,
    headers: &IndexMap<String, String>,
    method: &str,
    params: &P,
) -> Result<JsonRpcResult<R>, String>
where
    P: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let rpc_id = uuid::Uuid::new_v4().to_string();
    let envelope = serde_json::json!({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "method": method,
        "params": params,
    });

    let mut req = conn.http_client.post(&conn.url);
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("host")
            || k.eq_ignore_ascii_case("content-length")
            || k.eq_ignore_ascii_case("connection")
            || k.eq_ignore_ascii_case("accept")
            || k.eq_ignore_ascii_case("content-type")
            || k.eq_ignore_ascii_case("mcp-session-id")
        {
            continue;
        }
        req = req.header(k, v);
    }
    req = req
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Mcp-Session-Id", &conn.session_id)
        .json(&envelope);

    let resp = req
        .send()
        .await
        .map_err(|e| format!("forwarding HTTP request failed: {e}"))?;
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("reading response body failed: {e}"))?;
    let Some(body) = parse_json_or_sse(&resp_text) else {
        return Err("empty or unparseable upstream response".into());
    };

    if let Some(result) = body.get("result") {
        let typed: R = serde_json::from_value(result.clone())
            .map_err(|e| format!("decode upstream result: {e}"))?;
        return Ok(JsonRpcResult::Ok { result: typed });
    }
    if let Some(err) = body.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-32603);
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("upstream returned an error envelope without a message")
            .to_string();
        let data = err.get("data").cloned();
        return Ok(JsonRpcResult::Err { code, message, data });
    }
    Err("upstream response missing both `result` and `error`".into())
}

/// Streamable-HTTP responses may be plain JSON or an SSE body whose
/// `data:` lines carry the JSON — accept both.
fn parse_json_or_sse(text: &str) -> Option<serde_json::Value> {
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return Some(v);
    }
    let collected: String = text
        .lines()
        .filter_map(|l| l.strip_prefix("data: ").or_else(|| l.strip_prefix("data:")))
        .collect();
    if collected.is_empty() {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(&collected).ok()
}
