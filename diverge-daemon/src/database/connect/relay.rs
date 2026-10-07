//! Everything after the handshake, relayed unread.

use std::time::Instant;

use bytes::{Bytes, BytesMut};
use postgres_protocol::message::frontend;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::watch;

use super::Server;

/// How much of the database's writing is read at once.
const READ_CHUNK: usize = 16 * 1024;

/// Relay until either end ends: what the container writes to the
/// database as it comes, the daemon keeping count of message
/// boundaries by the five-byte headers and reading nothing else, so
/// that the container's socket ending is a `Terminate` when the
/// stream was at a boundary and the socket closed otherwise; what the
/// database says to the container as it comes, its end the container
/// socket closed. Every piece either way is a use of the container.
pub async fn relay(server: Server, mut from_container: UnboundedReceiver<Bytes>, to_container: UnboundedSender<Bytes>, touched: watch::Sender<Instant>) {
    let (mut reading, mut writing) = tokio::io::split(server);
    let touching = touched.clone();
    let outbound = async move {
        let mut remaining = 0usize;
        let mut header = Vec::with_capacity(5);
        while let Some(bytes) = from_container.recv().await {
            touching.send_replace(Instant::now());
            if writing.write_all(&bytes).await.is_err() {
                return;
            }
            boundaries(&bytes, &mut remaining, &mut header);
        }
        if remaining == 0 && header.is_empty() {
            let mut terminate = BytesMut::new();
            frontend::terminate(&mut terminate);
            let _ = writing.write_all(&terminate).await;
        }
        let _ = writing.shutdown().await;
    };
    let inbound = async move {
        let mut buffer = vec![0u8; READ_CHUNK];
        loop {
            match reading.read(&mut buffer).await {
                Ok(0) | Err(_) => return,
                Ok(read) => {
                    touched.send_replace(Instant::now());
                    if to_container.send(Bytes::copy_from_slice(&buffer[..read])).is_err() {
                        return;
                    }
                }
            }
        }
    };
    tokio::join!(outbound, inbound);
}

/// Walk `bytes` through the container's message boundaries:
/// `remaining` is how much of the current message is still to come,
/// `header` the bytes of the next header seen so far when a header
/// was split across pieces.
fn boundaries(bytes: &[u8], remaining: &mut usize, header: &mut Vec<u8>) {
    let mut rest = bytes;
    while !rest.is_empty() {
        if *remaining > 0 {
            let taken = (*remaining).min(rest.len());
            *remaining -= taken;
            rest = &rest[taken..];
            continue;
        }
        let needed = 5 - header.len();
        let taken = needed.min(rest.len());
        header.extend_from_slice(&rest[..taken]);
        rest = &rest[taken..];
        if header.len() == 5 {
            let length = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
            *remaining = usize::try_from(length).unwrap_or(0).saturating_sub(4);
            header.clear();
        }
    }
}
