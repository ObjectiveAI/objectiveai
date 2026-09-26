//! A read scope: one file out of the container.

use diverge_sdk::CHUNK_SIZE;
use diverge_sdk::container_proxy::outside::filesystem::read::client::request;
use diverge_sdk::container_proxy::outside::filesystem::read::server::response;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers;
use diverge_sdk::shared::error::Error;
use tokio::io::AsyncReadExt as _;

use crate::encode::encoded;
use crate::paths;

/// Serve one read until the file is sent or the read is over.
///
/// The rules, in order:
///
/// 1. The path must name a file ([`paths::absolute`]), else `Error`.
/// 2. The file is opened as the OS opens it, a symlink followed, and
///    must then be a regular file — a directory above all is never
///    read, per the shared read's doctrine. An open that fails, or
///    anything else, is `Error`, with the reason.
/// 3. The bytes go out as bodies, [`CHUNK_SIZE`] at most per frame,
///    until a read returns nothing. A file that had nothing is one
///    empty body: the finish alone would mean refused. A read that
///    fails mid-file is `Error` after the bodies already sent — the
///    file was not read whole, and this is why.
/// 4. The finish: the read complete, or its error delivered.
///
/// The server opens no channel on a read, so nothing here listens
/// for it.
pub async fn read(scope: ScopeHandle, frame: request::Frame) {
    let Some(path) = paths::absolute(&frame.0.path) else {
        error(&scope, "path", "path names no file").await;
        return;
    };

    let mut file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(reason) => {
            error(&scope, "open", &reason.to_string()).await;
            return;
        }
    };
    if !file.metadata().await.is_ok_and(|meta| meta.is_file()) {
        error(&scope, "open", "not a regular file").await;
        return;
    }

    let mut buffer = vec![0; CHUNK_SIZE];
    let mut sent = false;
    loop {
        match file.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                send(&scope, containers::read::response::Frame(&buffer[..n])).await;
                sent = true;
            }
            Err(reason) => {
                error(&scope, "read", &reason.to_string()).await;
                return;
            }
        }
    }
    if !sent {
        send(&scope, containers::read::response::Frame(&[])).await;
    }
    scope.send_response_finish().await;
}

/// One body, as the scope's response.
async fn send(scope: &ScopeHandle, body: containers::read::response::Frame<'_>) {
    if let Some(payload) = encoded(&response::Frame::Body(body)) {
        scope.send_response(&payload).await;
    }
}

/// The error, then the finish.
async fn error(scope: &ScopeHandle, kind: &str, reason: &str) {
    let error = Error(serde_json::json!({
        "kind": kind,
        "error": reason,
    }));
    if let Some(payload) = encoded(&response::Frame::Error(error)) {
        scope.send_response(&payload).await;
    }
    scope.send_response_finish().await;
}
