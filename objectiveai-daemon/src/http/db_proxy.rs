//! Raw TCP PostgreSQL proxy.
//!
//! A plain TCP listener (its OWN random system port, not a path on the
//! HTTP server) that transparently forwards the PostgreSQL wire
//! protocol to the daemon's configured cluster: a stock client (psql,
//! any driver) points at the proxy's `host:port` and connects exactly
//! as if it were native Postgres. Bytes flow through opaquely — no
//! protocol parsing, no query handling — so the client's own SSL
//! negotiation and authentication are end-to-end with the real
//! cluster (auth is Postgres's own; the proxy adds none).
//!
//! The upstream address is resolved PER CONNECTION via
//! [`crate::context::GlobalContext::db_handle`], which starts (or
//! respawns) the local embedded cluster if needed and returns the
//! current `host:port` — so a `db config` change or a bounced local
//! cluster is picked up on the next connection with nothing cached
//! here.

use tokio::net::{TcpListener, TcpStream};

/// Accept loop: one spawned task per inbound client, each a
/// bidirectional byte pipe to the current Postgres address. Runs for
/// the daemon's life. An accept error ends the loop (the listener is
/// gone); per-connection errors are isolated to their task.
pub async fn serve(global: crate::context::GlobalContext, listener: TcpListener) {
    loop {
        let inbound = match listener.accept().await {
            Ok((stream, _peer)) => stream,
            Err(_) => return,
        };
        let global = global.clone();
        tokio::spawn(async move {
            proxy_one(global, inbound).await;
        });
    }
}

async fn proxy_one(global: crate::context::GlobalContext, mut inbound: TcpStream) {
    // Resolve (and ensure alive) the current cluster address. On
    // failure the client just sees the connection close — the same as
    // an unreachable Postgres.
    let address = match global.db_handle().await {
        Ok(handle) => handle.address,
        Err(_) => return,
    };
    let mut outbound = match TcpStream::connect(&address).await {
        Ok(stream) => stream,
        Err(_) => return,
    };
    // Postgres is latency-sensitive (small request/response turns);
    // Nagle off on both halves. Keepalive matches the rest of the
    // daemon's sockets so a silently-dead peer surfaces as an error.
    let _ = inbound.set_nodelay(true);
    let _ = outbound.set_nodelay(true);
    objectiveai_sdk::net::set_tcp_keepalive(&inbound);
    objectiveai_sdk::net::set_tcp_keepalive(&outbound);
    // Pure passthrough until either side closes.
    let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
}
