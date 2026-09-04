//! One connection from the agent's driver, carried to the server.

use std::pin::pin;
use std::sync::Arc;

use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::postgres_proxy;
use futures_util::future;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

use crate::conduit::{Conduit, Outbound};

/// How much of the agent's writing is read at once. A read size, not
/// a protocol limit: a pgwire message larger than this leaves as
/// several frames, and the far end reassembles as from a socket.
const READ_CHUNK: usize = 16 * 1024;

/// Serve one accepted socket until it ends, from either side.
///
/// Waits for a WebSocket (the park), registers the connection on it,
/// announces it, then pumps: a write task feeds the socket with what
/// the server sends for this connection, and this task reads the
/// agent's bytes into `data` frames until the agent closes, the read
/// fails, or the WebSocket goes away — whichever comes first.
///
/// On the way out the connection is closed on this side and, if the
/// WebSocket it rode is still there, a `close` frame tells the server
/// the agent's socket ended. The write pump is dropped, not awaited:
/// awaiting it would block on an agent that stopped reading, and a
/// dropped pump still shuts the socket, since the pump owns the write
/// half and the drop of the sender behind it ends its loop.
pub async fn serve(stream: TcpStream, conduit: Arc<Conduit>) {
    // pgwire is chatty in small messages; Nagle would only add
    // latency to each one.
    let _ = stream.set_nodelay(true);

    // Wait for a WebSocket, and register on it — again, if it went
    // between the wait and the registration.
    let (outbound, id, mut inbound) = loop {
        let outbound = conduit.attached().await;
        if let Some((id, inbound)) = conduit.open(outbound.generation).await {
            break (outbound, id, inbound);
        }
        conduit.detached(outbound.generation).await;
    };

    if !send(&outbound, &postgres_proxy::container::Frame::Open { connection: id })
        .await
    {
        conduit.close(id).await;
        return;
    }

    let (mut read, mut write) = stream.into_split();
    let writer = tokio::spawn(async move {
        while let Some(bytes) = inbound.recv().await {
            if write.write_all(&bytes).await.is_err() {
                break;
            }
        }
        let _ = write.shutdown().await;
    });

    let mut buffer = vec![0; READ_CHUNK];
    let mut detached = pin!(conduit.detached(outbound.generation));
    loop {
        let reading = pin!(read.read(&mut buffer));
        match future::select(&mut detached, reading).await {
            // The WebSocket went: this connection died with it.
            future::Either::Left(((), _)) => break,
            // The agent closed, or the socket failed under it.
            future::Either::Right((Ok(0), _))
            | future::Either::Right((Err(_), _)) => break,
            future::Either::Right((Ok(n), _)) => {
                let frame = postgres_proxy::container::Frame::Data {
                    connection: id,
                    payload: &buffer[..n],
                };
                if !send(&outbound, &frame).await {
                    break;
                }
            }
        }
    }

    conduit.close(id).await;
    // Fails harmlessly when the WebSocket is the thing that ended.
    send(&outbound, &postgres_proxy::container::Frame::Close { connection: id })
        .await;
    drop(writer);
}

/// Encode one frame and queue it toward the WebSocket. `false` is the
/// WebSocket gone; the encode itself cannot fail.
async fn send(
    outbound: &Outbound,
    frame: &postgres_proxy::container::Frame<'_>,
) -> bool {
    let mut bytes = Vec::new();
    frame
        .encode(&mut Writer::new(&mut bytes))
        .unwrap_or_else(|error| match error {});
    outbound.send(bytes).await
}
