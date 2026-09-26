//! Postgres, from the inside: the loopback listener the driver dials,
//! each socket announced and then carried.
//!
//! The proxy is a database to the container. A driver that dials
//! `127.0.0.1:81` gets a socket that is one ask on the begin scope —
//! `Postgres`, with an id the proxy minted — and, once the server
//! opens its half, a conduit: the driver's bytes out as responses on
//! the server's channel, the ask's frames in as the driver's bytes,
//! nothing parsed, until either side hangs up.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use bytes::Bytes;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::postgres;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, oneshot};

use crate::answer::{self, Answer};
use crate::ask;
use crate::own::Own;
use crate::proxy::Proxy;

/// How much of the driver's writing is read at once. A read size, not
/// a protocol limit: a pgwire message larger than this leaves as
/// several frames, and the far end reassembles as from a socket.
const READ_CHUNK: usize = 16 * 1024;

/// How long the accept loop pauses after an accept that failed —
/// descriptors exhausted, most likely — before trying again, so a
/// failure that persists is a slow loop rather than a hot one.
const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

/// The connections announced and not yet paired: each waits for the
/// server to open its half, quoting the id, and is handed the
/// channel it did.
pub struct Pairs {
    next: AtomicU32,
    pending: Mutex<HashMap<u32, oneshot::Sender<u32>>>,
}

impl Pairs {
    pub fn new() -> Self {
        Pairs {
            // From one: unique among the connections not yet paired is
            // all that is needed, and never reused is the simplest
            // way to be that.
            next: AtomicU32::new(1),
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// A fresh id, and where the server's half arrives.
    async fn announce(&self) -> (u32, oneshot::Receiver<u32>) {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(id, sender);
        (id, receiver)
    }

    /// The server's half arrived for `id`, on `channel`: hand it over.
    /// `false` is an id nobody is waiting on.
    async fn pair(&self, id: u32, channel: u32) -> bool {
        let Some(sender) = self.pending.lock().await.remove(&id) else {
            return false;
        };
        sender.send(channel).is_ok()
    }

    /// The announcement is withdrawn: the ask ended before the half
    /// came.
    async fn forget(&self, id: u32) {
        self.pending.lock().await.remove(&id);
    }
}

/// The server opened its half of `connection_id` on `channel`: hand
/// the channel to the connection waiting for it. An id nobody waits
/// on is a channel this end cannot serve, finished with nothing.
pub async fn attach(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, channel: u32, connection_id: u32) {
    if !proxy.pairs.pair(connection_id, channel).await {
        scope.send_channel_response_finish(channel).await;
    }
}

/// Accept the driver's connections for the proxy's whole life, one
/// task each.
pub async fn accept(listener: TcpListener, proxy: Arc<Proxy>) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(connection(stream, Arc::clone(&proxy)));
            }
            Err(_) => tokio::time::sleep(ACCEPT_PAUSE).await,
        }
    }
}

/// Serve one accepted socket until it ends, from either side.
///
/// Announces it — which waits for the connection to begin if it has
/// not, the park — and then waits for the server's half, reading
/// NOTHING from the driver until it has: pgwire is client-first, and
/// the startup message the driver wrote on connecting waits in the
/// kernel's buffer. An announcement the server finishes unopened, or
/// that dies, is the socket dropped, which the driver sees as a
/// server that hung up; a pool reconnects, and that is a new
/// announcement.
///
/// Once paired, a writer task feeds the socket with every frame the
/// ask carries and shuts it when the ask ends, and this task reads
/// the driver's bytes onto the server's half until the driver closes
/// or the read fails, and finishes the half — how the server learns
/// the driver hung up. The writer is dropped rather than awaited:
/// awaiting it would block on a driver that stopped reading, and a
/// dropped writer still shuts the socket, since it owns the write
/// half and ends with the ask.
async fn connection(stream: TcpStream, proxy: Arc<Proxy>) {
    // pgwire is chatty in small messages; Nagle would only add
    // latency to each one.
    let _ = stream.set_nodelay(true);

    let (id, half) = proxy.pairs.announce().await;
    let Ok((begun, mut channel)) = ask::open(
        &proxy,
        Own::Postgres(postgres::request::Postgres { connection_id: id }),
    )
    .await
    else {
        proxy.pairs.forget(id).await;
        return;
    };

    // Frames the database says before the half is open — pgwire is
    // client-first, so none is expected — are kept for the writer.
    let mut early = Vec::new();
    let mut half = std::pin::pin!(half);
    let server_channel = loop {
        tokio::select! {
            paired = &mut half => match paired {
                Ok(channel) => break channel,
                Err(_) => {
                    proxy.pairs.forget(id).await;
                    return;
                }
            },
            next = answer::next(&mut channel) => match next {
                Some(Answer::Frame(bytes)) => early.push(bytes),
                Some(Answer::Finish) | None => {
                    proxy.pairs.forget(id).await;
                    return;
                }
            },
        }
    };

    let (mut read, mut write) = stream.into_split();
    let writer = tokio::spawn(async move {
        for bytes in early {
            if write.write_all(&bytes).await.is_err() {
                let _ = write.shutdown().await;
                return;
            }
        }
        loop {
            match answer::next(&mut channel).await {
                Some(Answer::Frame(bytes)) => {
                    if write.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                Some(Answer::Finish) | None => break,
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
                begun.scope.send_channel_response(server_channel, &chunk).await;
            }
        }
    }
    begun.scope.send_channel_response_finish(server_channel).await;

    drop(writer);
}
