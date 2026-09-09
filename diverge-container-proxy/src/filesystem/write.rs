//! `/write`, served: one file into the container.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::ws::{Message, WebSocket};
use diverge_provider_sdk::container_proxy::write;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::fs;
use tokio::io::AsyncWriteExt as _;

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
/// 1. The first message is the ask. It must be binary and decode as
///    the request; anything else is a refusal — the clean close with
///    nothing before it — since the server sent something this is
///    not.
/// 2. The path must name a file ([`paths::absolute`]), else `Error`.
/// 3. The temporary is created new beside the destination; a parent
///    that is missing, or is not a directory, fails here, and that
///    is the `Error`.
/// 4. Every non-empty binary message is content, written in order;
///    a write that fails discards the temporary and is the `Error`.
///    The EMPTY message is the end of the content. A close, a text
///    message, an error or the end before it is the write abandoned
///    or dead: the temporary is discarded, the socket dropped, and
///    the destination is untouched.
/// 5. The handle is dropped, the destination's permissions copied
///    if it is a regular file, and the temporary renamed over the
///    destination — atomic on one filesystem, and the parent is the
///    same directory by construction. A rename that fails (the
///    destination is a directory, the filesystem refused) discards
///    the temporary and is the `Error`; otherwise `Ok`. The answer
///    is sent and the socket closed cleanly. No fsync: durability
///    past the container's life is not this wire's promise.
pub async fn serve(mut socket: WebSocket) {
    let request = match socket.next().await {
        Some(Ok(Message::Binary(bytes))) => {
            write::request::Request::decode(&bytes).ok()
        }
        _ => None,
    };
    let Some(request) = request else {
        let _ = socket.close().await;
        return;
    };
    let Some(destination) = paths::absolute(&request.path) else {
        answer(socket, write::response::Frame::Error("path names no file")).await;
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
        Err(error) => {
            let reason = format!("create: {error}");
            answer(socket, write::response::Frame::Error(&reason)).await;
            return;
        }
    };

    loop {
        match socket.next().await {
            Some(Ok(Message::Binary(bytes))) if bytes.is_empty() => break,
            Some(Ok(Message::Binary(bytes))) => {
                let chunk = write::request::Frame::decode(&bytes)
                    .unwrap_or_else(|error| match error {});
                if let Err(error) = file.write_all(chunk.0).await {
                    drop(file);
                    discard(&temporary).await;
                    let reason = format!("write: {error}");
                    answer(socket, write::response::Frame::Error(&reason)).await;
                    return;
                }
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            _ => {
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
        Ok(()) => answer(socket, write::response::Frame::Ok).await,
        Err(error) => {
            discard(&temporary).await;
            let reason = format!("finish: {error}");
            answer(socket, write::response::Frame::Error(&reason)).await;
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

/// Send the one answer and close cleanly. A socket that is gone
/// takes the answer with it; there is nobody left to tell.
async fn answer(mut socket: WebSocket, frame: write::response::Frame<'_>) {
    let mut out = Vec::new();
    frame
        .encode(&mut Writer::new(&mut out))
        .unwrap_or_else(|error| match error {});
    if socket.send(Message::Binary(out.into())).await.is_ok() {
        let _ = socket.close().await;
    }
}
