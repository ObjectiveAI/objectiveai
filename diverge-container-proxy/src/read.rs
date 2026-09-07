//! `/read`, served: one file out of the container.

use axum::extract::ws::{Message, WebSocket};
use diverge_provider_sdk::CHUNK_SIZE;
use diverge_provider_sdk::container_proxy::read;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::io::AsyncReadExt as _;

use crate::paths;

/// Serve one read until the file is sent or the read is over.
///
/// The rules, in order:
///
/// 1. The first message is the ask. It must be binary and decode as
///    the request; a text message, a close, an error, the end, or a
///    body that will not decode is a refusal — the clean close with
///    nothing before it — since the server sent something this is
///    not.
/// 2. The path must name a file: [`paths::absolute`], or a refusal.
/// 3. The file is opened as the OS opens it, a symlink followed, and
///    must then be a regular file — a directory above all is never
///    read, per the shared read's doctrine. An open that fails, or
///    anything else, is a refusal.
/// 4. The bytes go out as they are read, [`CHUNK_SIZE`] at most per
///    message, until a read returns nothing. A file that had nothing
///    is one empty message: the clean close alone would mean refused.
///    A read that fails mid-file is the read dying, and the socket is
///    dropped unclosed — the abrupt end the wire names for it. A send
///    that fails is the server gone.
/// 5. The clean close: the read complete.
///
/// The server is silent after its one message, so nothing here
/// listens for it: a server that went away is a send that fails.
pub async fn serve(mut socket: WebSocket) {
    let request = match socket.next().await {
        Some(Ok(Message::Binary(bytes))) => {
            read::request::Request::decode(&bytes).ok()
        }
        _ => None,
    };
    let Some(path) =
        request.and_then(|request| paths::absolute(&request.path))
    else {
        let _ = socket.close().await;
        return;
    };

    let Ok(mut file) = tokio::fs::File::open(&path).await else {
        let _ = socket.close().await;
        return;
    };
    if !file.metadata().await.is_ok_and(|meta| meta.is_file()) {
        let _ = socket.close().await;
        return;
    }

    let mut buffer = vec![0; CHUNK_SIZE];
    let mut sent = false;
    loop {
        match file.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                if !send(&mut socket, &buffer[..n]).await {
                    return;
                }
                sent = true;
            }
            Err(_) => return,
        }
    }
    if !sent && !send(&mut socket, &[]).await {
        return;
    }
    let _ = socket.close().await;
}

/// One piece of the file, as the wire's frame. `false` is the socket
/// gone.
async fn send(socket: &mut WebSocket, bytes: &[u8]) -> bool {
    let mut out = Vec::with_capacity(bytes.len());
    read::response::Frame(bytes)
        .encode(&mut Writer::new(&mut out))
        .unwrap_or_else(|error| match error {});
    socket.send(Message::Binary(out.into())).await.is_ok()
}
