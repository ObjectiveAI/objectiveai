//! `/filesystem/read`, served: one file out of the container.

use axum::extract::ws::{Message, WebSocket};
use diverge_provider_sdk::CHUNK_SIZE;
use diverge_provider_sdk::container_proxy::filesystem::read;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use diverge_provider_sdk::shared::containers;
use futures_util::SinkExt as _;
use tokio::io::AsyncReadExt as _;

use crate::paths;
use crate::ws;

/// Serve one read until the file is sent or the read is over.
///
/// The rules, in order:
///
/// 1. The first message is the ask. It must be binary and decode as
///    the request; a close, an error, the end, or a body that will
///    not decode is the clean close with nothing before it — the
///    server sent something this is not, and there is no read to say
///    anything about. A text frame is passed over.
/// 2. The path must name a file ([`paths::absolute`]), else `Error`.
/// 3. The file is opened as the OS opens it, a symlink followed, and
///    must then be a regular file — a directory above all is never
///    read, per the shared read's doctrine. An open that fails, or
///    anything else, is `Error`, with the reason.
/// 4. The bytes go out as bodies, [`CHUNK_SIZE`] at most per message,
///    until a read returns nothing. A file that had nothing is one
///    empty body: the clean close alone would mean refused. A read
///    that fails mid-file is `Error` after the bodies already sent —
///    the file was not read whole, and this is why. A send that fails
///    is the server gone.
/// 5. The clean close: the read complete, or its error delivered.
///
/// The server is silent after its one message, so nothing here
/// listens for it: a server that went away is a send that fails.
pub async fn serve(mut socket: WebSocket) {
    let request = ws::binary(&mut socket)
        .await
        .and_then(|bytes| read::request::Request::decode(&bytes).ok());
    let Some(request) = request else {
        let _ = socket.close().await;
        return;
    };
    let Some(path) = paths::absolute(&request.path) else {
        error(socket, "path names no file").await;
        return;
    };

    let mut file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(reason) => {
            error(socket, &format!("open: {reason}")).await;
            return;
        }
    };
    if !file.metadata().await.is_ok_and(|meta| meta.is_file()) {
        error(socket, "not a regular file").await;
        return;
    }

    let mut buffer = vec![0; CHUNK_SIZE];
    let mut sent = false;
    loop {
        match file.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                let body = containers::read::response::Frame(&buffer[..n]);
                if !send(&mut socket, read::response::Frame::Body(body)).await {
                    return;
                }
                sent = true;
            }
            Err(reason) => {
                error(socket, &format!("read: {reason}")).await;
                return;
            }
        }
    }
    if !sent {
        let body = containers::read::response::Frame(&[]);
        if !send(&mut socket, read::response::Frame::Body(body)).await {
            return;
        }
    }
    let _ = socket.close().await;
}

/// The error, then the clean close.
async fn error(mut socket: WebSocket, reason: &str) {
    if send(&mut socket, read::response::Frame::Error(reason)).await {
        let _ = socket.close().await;
    }
}

/// One message, as the wire's frame. `false` is the socket gone.
async fn send(socket: &mut WebSocket, frame: read::response::Frame<'_>) -> bool {
    let mut out = Vec::new();
    frame
        .encode(&mut Writer::new(&mut out))
        .unwrap_or_else(|error| match error {});
    socket.send(Message::Binary(out.into())).await.is_ok()
}
