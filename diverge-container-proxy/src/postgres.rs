//! Postgres, from the inside: the loopback listener the driver dials,
//! each socket announced and then carried.
//!
//! The proxy is a database to the container. A driver that dials
//! `127.0.0.1:81` gets a socket that is one ask on `/requests` —
//! kind `11`, the channel and nothing else — and, once the server
//! opens `/postgres/{channel}`, a conduit: the driver's bytes out as
//! the path's messages, the path's messages in as the driver's bytes,
//! nothing parsed, until either side hangs up.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use diverge_provider_sdk::container_proxy::postgres;
use diverge_provider_sdk::container_proxy::requests::request::Request;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

use crate::requests::{Event, Requests};

/// How much of the driver's writing is read at once. A read size, not
/// a protocol limit: a pgwire message larger than this leaves as
/// several messages, and the far end reassembles as from a socket.
const READ_CHUNK: usize = 16 * 1024;

/// How long the accept loop pauses after an accept that failed —
/// descriptors exhausted, most likely — before trying again, so a
/// failure that persists is a slow loop rather than a hot one.
const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

/// Accept the driver's connections for the proxy's whole life, one
/// task each.
pub async fn accept(listener: TcpListener, requests: Arc<Requests>) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(connection(stream, Arc::clone(&requests)));
            }
            Err(_) => tokio::time::sleep(ACCEPT_PAUSE).await,
        }
    }
}

/// Serve one accepted socket until it ends, from either side.
///
/// Announces it — which waits for a `/requests` connection if none is
/// live, the park — and then waits for the path to open, reading
/// NOTHING from the driver until it has: pgwire is client-first, and
/// the startup message the driver wrote on connecting waits in the
/// kernel's buffer. An announcement that dies unopened is the socket
/// dropped, which the driver sees as a server that hung up; a pool
/// reconnects, and that is a new announcement.
///
/// Once open, a writer task feeds the socket with every message the
/// path carries and shuts it when the path ends, and this task reads
/// the driver's bytes toward the path until the driver closes, the
/// read fails, or the path is gone — a send that fails, because the
/// path's pump dropped its queue. On the way out the sender is
/// dropped, which is how the path learns the driver hung up, and the
/// writer is dropped rather than awaited: awaiting it would block on
/// a driver that stopped reading, and a dropped writer still shuts
/// the socket, since it owns the write half and ends with the path.
async fn connection(stream: TcpStream, requests: Arc<Requests>) {
    // pgwire is chatty in small messages; Nagle would only add
    // latency to each one.
    let _ = stream.set_nodelay(true);

    let Ok((_, mut events)) = requests
        .ask(Request::Postgres(postgres::request::Request))
        .await
    else {
        // The announcement would not encode, which it cannot: it is
        // empty. Nothing to do for the driver but hang up.
        return;
    };

    let sender = match events.recv().await {
        Some(Event::Opened(sender)) => sender,
        Some(Event::Message(_) | Event::Complete | Event::Died) | None => {
            return;
        }
    };

    let (mut read, mut write) = stream.into_split();
    let writer = tokio::spawn(async move {
        loop {
            match events.recv().await {
                Some(Event::Message(bytes)) => {
                    if write.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                // Already heard, once, above.
                Some(Event::Opened(_)) => {}
                Some(Event::Complete | Event::Died) | None => break,
            }
        }
        let _ = write.shutdown().await;
    });

    let mut buffer = vec![0; READ_CHUNK];
    loop {
        match read.read(&mut buffer).await {
            // The driver closed, or the socket failed under it.
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let chunk = Bytes::copy_from_slice(&buffer[..n]);
                if sender.send(chunk).await.is_err() {
                    break;
                }
            }
        }
    }

    drop(sender);
    drop(writer);
}
