//! The laboratory transfer registry — parked `/export` streams and
//! `/import` bodies, keyed by manager-minted transfer ids.
//!
//! Moved shape-verbatim out of the per-laboratory server so BOTH
//! laboratory kinds can own one: regular [`LabServer`](crate::server::LabServer)s
//! and [`EphemeralLab`](crate::ephemeral::EphemeralLab)s (transfers are
//! id-agnostic — they work against any live laboratory, and against an
//! ephemeral they last only as long as the laboratory does).

use std::sync::Arc;

use dashmap::DashMap;
use objectiveai_sdk::laboratories::daemon::{
    ExportBeginRequest, ExportChunk, ImportBeginRequest, ImportEndResult,
    ImportWriteRequest, JsonRpcResult, ResponsePayload, TransferAck,
    TransferBeginResult, TransferIdRequest,
};

use crate::upstream::rpc_err;

/// Raw bytes per `LaboratoryExportRead` chunk (base64 on the wire).
const TRANSFER_CHUNK_SIZE: usize = 2 * 1024 * 1024;

/// A transfer half untouched this long was abandoned by its driver —
/// swept lazily on every Begin.
const TRANSFER_IDLE_SECS: i64 = 300;

/// One parked laboratory-transfer half.
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

/// One laboratory's transfer registry, bound to the container's
/// loopback HTTP base.
pub struct Transfers {
    base_url: String,
    entries: DashMap<String, Arc<TransferEntry>>,
}

impl Transfers {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            entries: DashMap::new(),
        }
    }

    /// Whether any transfer half is parked — in-flight transfers are
    /// regular-laboratory demand (an idle stop mid-stream would
    /// truncate them). Ephemeral lifetime ignores this by design.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn gc(&self) {
        self.entries
            .retain(|_, entry| entry.idle_secs() < TRANSFER_IDLE_SECS);
    }

    pub async fn export_begin(&self, req: ExportBeginRequest) -> ResponsePayload {
        let err = |m: String| ResponsePayload::ExportBegin(rpc_err(-32603, m));
        self.gc();
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
        self.entries.insert(transfer_id.clone(), Arc::new(entry));
        ResponsePayload::ExportBegin(JsonRpcResult::Ok {
            result: TransferBeginResult { transfer_id },
        })
    }

    pub async fn export_read(&self, req: TransferIdRequest) -> ResponsePayload {
        use base64::Engine as _;
        let err = |m: String| ResponsePayload::ExportRead(rpc_err(-32603, m));
        let entry = match self.entries.get(&req.transfer_id) {
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
                    self.entries.remove(&req.transfer_id);
                    return err(format!("export stream: {e}"));
                }
            }
        }
        if eof {
            *guard = None;
            drop(guard);
            self.entries.remove(&req.transfer_id);
        }
        ResponsePayload::ExportRead(JsonRpcResult::Ok {
            result: ExportChunk {
                data: base64::engine::general_purpose::STANDARD.encode(&buf),
                eof,
            },
        })
    }

    pub fn export_abort(&self, req: TransferIdRequest) -> ResponsePayload {
        self.entries.remove(&req.transfer_id);
        ResponsePayload::ExportAbort(JsonRpcResult::Ok {
            result: TransferAck {},
        })
    }

    pub async fn import_begin(&self, req: ImportBeginRequest) -> ResponsePayload {
        self.gc();
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
        self.entries.insert(transfer_id.clone(), Arc::new(entry));
        ResponsePayload::ImportBegin(JsonRpcResult::Ok {
            result: TransferBeginResult { transfer_id },
        })
    }

    pub async fn import_write(&self, req: ImportWriteRequest) -> ResponsePayload {
        use base64::Engine as _;
        let err = |m: String| ResponsePayload::ImportWrite(rpc_err(-32603, m));
        let entry = match self.entries.get(&req.transfer_id) {
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
            self.entries.remove(&req.transfer_id);
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

    pub async fn import_end(&self, req: TransferIdRequest) -> ResponsePayload {
        let err = |m: String| ResponsePayload::ImportEnd(rpc_err(-32603, m));
        let entry = match self.entries.remove(&req.transfer_id) {
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

    pub fn import_abort(&self, req: TransferIdRequest) -> ResponsePayload {
        if let Some((_, entry)) = self.entries.remove(&req.transfer_id) {
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

/// The local-transfer fast path: pipe one laboratory's `/export`
/// stream STRAIGHT into another's `/import` body — no chunk staging,
/// no base64, no parked transfer entries; reqwest streams the tar end
/// to end and the HTTP bodies provide the backpressure. Returns the
/// byte total the destination ingested. Dropping the export response
/// on any failure aborts the GET, so nothing leaks. Free over the two
/// base URLs so any mix of regular/ephemeral endpoints works.
pub async fn pipe_export(
    source_base_url: &str,
    source_path: &str,
    destination_base_url: &str,
    destination_path: &str,
) -> Result<u64, String> {
    let export = reqwest::Client::new()
        .get(format!("{source_base_url}/export"))
        .query(&[("path", &source_path)])
        .send()
        .await
        .map_err(|e| format!("export: {e}"))?;
    if !export.status().is_success() {
        let status = export.status();
        let body = export.text().await.unwrap_or_default();
        return Err(format!("export: HTTP {status}: {}", body.trim()));
    }
    // Count bytes as they flow — the import side reports only HTTP
    // success, and the export stream is consumed exactly once, here.
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
        .post(format!("{destination_base_url}/import"))
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
