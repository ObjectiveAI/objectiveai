//! A write scope: one file into the container, its content on a
//! channel the proxy opens.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use diverge_sdk::container_proxy::outside::endpoints::filesystem::write::client::{channel_response, request};
use diverge_sdk::container_proxy::outside::endpoints::filesystem::write::server::{channel_request, response};
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::write_path;
use diverge_sdk::shared::error::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt as _;

use crate::answer::{self, Answer};
use crate::encode::encoded;
use crate::paths;

/// Counted up per temporary, so two writes to one destination at
/// once get two temporaries; `create_new` catches whatever is left.
static TEMPORARIES: AtomicU64 = AtomicU64::new(0);

/// Serve one write until the file is in place or the write is over.
///
/// The destination is never written directly: a half-written file
/// that looks finished is the outcome the wire names as the worse
/// one. The content goes to a temporary beside the destination —
/// `.<name>.diverge-write-<n>` — and one rename moves it into place
/// at the end, so the destination is either what it was or the whole
/// new file. A symlink there is replaced, not written through; a
/// regular file there lends the new one its permissions.
///
/// The rules, in order:
///
/// 1. The path must name a file ([`paths::absolute`]), else `Error`.
/// 2. The temporary is created new beside the destination; a parent
///    that is missing, or is not a directory, fails here, and that
///    is the `Error`.
/// 3. The content channel is opened — the proxy's one ask on a write,
///    carrying nothing — and every body on it is written in order; a
///    write that fails discards the temporary and is the `Error`.
///    The channel's finish is the end of the content. An error on it
///    is the content not supplied: the temporary is discarded, and
///    that is the `Error`. The channel closing without a finish is
///    the connection gone: the temporary is discarded, and nothing is
///    answered.
/// 4. The handle is dropped, the destination's permissions copied
///    if it is a regular file, and the temporary renamed over the
///    destination — atomic on one filesystem, and the parent is the
///    same directory by construction. A rename that fails (the
///    destination is a directory, the filesystem refused) discards
///    the temporary and is the `Error`; otherwise `Written`. Then the
///    finish. No fsync: durability past the container's life is not
///    this wire's promise.
pub async fn write(scope: ScopeHandle, frame: request::Frame) {
    let Some(destination) = paths::absolute(&frame.path) else {
        error(&scope, "path", "path names no file").await;
        return;
    };

    let temporary = temporary(&destination);
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .await
    {
        Ok(file) => file,
        Err(reason) => {
            error(&scope, "create", &reason.to_string()).await;
            return;
        }
    };

    let ask = encoded(&channel_request::Frame).unwrap_or_default();
    let mut content = scope.send_channel_request(&ask).await;
    loop {
        match answer::next(&mut content).await {
            Some(Answer::Frame(bytes)) => match channel_response::content::Frame::decode(&bytes) {
                Ok(channel_response::content::Frame::Body(body)) => {
                    if let Err(reason) = file.write_all(body.0).await {
                        drop(file);
                        discard(&temporary).await;
                        error(&scope, "write", &reason.to_string()).await;
                        return;
                    }
                }
                Ok(channel_response::content::Frame::Error(reason)) => {
                    drop(file);
                    discard(&temporary).await;
                    if let Some(payload) = encoded(&response::Frame::Error(reason)) {
                        scope.send_response(&payload).await;
                    }
                    scope.send_response_finish().await;
                    return;
                }
                Err(reason) => {
                    drop(file);
                    discard(&temporary).await;
                    error(&scope, "content", &reason.to_string()).await;
                    return;
                }
            },
            Some(Answer::Finish) => break,
            None => {
                drop(file);
                discard(&temporary).await;
                return;
            }
        }
    }

    drop(file);
    if let Ok(meta) = fs::metadata(&destination).await
        && meta.is_file()
    {
        let _ = fs::set_permissions(&temporary, meta.permissions()).await;
    }
    match fs::rename(&temporary, &destination).await {
        Ok(()) => {
            if let Some(payload) = encoded(&response::Frame::Written(write_path::response::Frame)) {
                scope.send_response(&payload).await;
            }
            scope.send_response_finish().await;
        }
        Err(reason) => {
            discard(&temporary).await;
            error(&scope, "finish", &reason.to_string()).await;
        }
    }
}

/// The temporary's path: beside `destination`, named after it, with
/// a number no other temporary of this run has.
fn temporary(destination: &Path) -> PathBuf {
    let n = TEMPORARIES.fetch_add(1, Ordering::Relaxed);
    // `paths::absolute` guarantees a final component.
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    destination.with_file_name(format!(".{name}.diverge-write-{n}"))
}

/// Remove the temporary. One that is already gone is nothing to
/// remove.
async fn discard(temporary: &Path) {
    let _ = fs::remove_file(temporary).await;
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
