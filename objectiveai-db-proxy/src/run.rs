//! The two listeners.
//!
//! There are two sockets, and which way each one faces is the point:
//!
//! | role             | bind                 | protocol  |
//! |------------------|----------------------|-----------|
//! | Postgres clients | `127.0.0.1:14979`    | raw TCP   |
//! | laboratory host  | `0.0.0.0:14980`      | WebSocket |
//!
//! ALL FOUR VALUES ARE HARDCODED, and there is no configuration of any
//! kind — no env, no arguments, no `.env`. That is a deliberate
//! narrowing, not laziness.
//!
//! This binary is copied into a container somebody else built and
//! started with `podman exec`, which inherits that image's environment.
//! While these were read from env under generic names, an image
//! declaring `ENV ADDRESS=127.0.0.1` or `ENV PORT=8080` for its OWN
//! server — an entirely ordinary thing to do — silently reconfigured
//! the proxy, and binding the conduit port to loopback makes it
//! unreachable through a published port, so the plugin's database hung
//! with nothing to say why. Hardcoding removes that whole class of
//! failure instead of defending against it: there is nothing for an
//! image to collide with, and nothing the launcher has to remember to
//! pass.
//!
//! The Postgres side is loopback because nothing outside the container
//! has any business reaching it, and fixed because that is what lets a
//! connection string be stamped into container env before this process
//! exists — the whole reason a plugin can stay ignorant of the
//! mechanism. Inside a container's own network namespace there is no
//! contention to randomize away from.
//!
//! The host side binds `0.0.0.0` because a published container port does
//! not reach a loopback-bound listener.
//!
//! Neither port collides with anything already in these containers:
//! `14978` is the laboratory MCP server's. The laboratory host declares
//! the same two numbers independently, exactly as it declares the frame
//! format — the two halves agree by contract, not by configuration.

use std::sync::Arc;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::mpsc;

use crate::conduit::Conduit;
use crate::frame::{self, Frame};

/// Where Postgres clients connect. Loopback: in-container only.
pub const POSTGRES_ADDRESS: &str = "127.0.0.1";

/// The port Postgres clients connect to — what the launcher names in
/// `OBJECTIVEAI_POSTGRES_URL`.
pub const POSTGRES_PORT: u16 = 14979;

/// Where the laboratory host dials in. `0.0.0.0`, because a published
/// container port does not reach a loopback-bound listener.
pub const HOST_ADDRESS: &str = "0.0.0.0";

/// The port the laboratory host dials, published out of the container.
pub const HOST_PORT: u16 = 14980;

/// How much is read from a client socket at a time.
///
/// A read size, NOT a protocol limit: a Postgres message larger than
/// this simply spans several frames, because both ends of the conduit
/// reassemble from a byte stream exactly as they would from a socket.
const READ_CHUNK: usize = 16 * 1024;

pub struct Servers {
    pub conduit: Arc<Conduit>,
    pub postgres: tokio::net::TcpListener,
    pub host: tokio::net::TcpListener,
    pub router: axum::Router,
}

pub async fn setup() -> std::io::Result<Servers> {
    let conduit = Conduit::new();

    // A single route at the root: the port is dedicated to this and
    // nothing else, so a path would only be a second name for the same
    // thing that both ends have to agree on.
    let router = axum::Router::new()
        .route("/", axum::routing::any(upgrade))
        .with_state(Arc::clone(&conduit));

    // Two independent binds, so they go together rather than one after
    // the other. `try_join!` also gives the failure semantics this wants:
    // half a conduit is useless, so the first bind to fail is the whole
    // setup's error.
    let (postgres, host) = tokio::try_join!(
        tokio::net::TcpListener::bind((POSTGRES_ADDRESS, POSTGRES_PORT)),
        tokio::net::TcpListener::bind((HOST_ADDRESS, HOST_PORT)),
    )?;

    Ok(Servers {
        conduit,
        postgres,
        host,
        router,
    })
}

pub async fn serve(servers: Servers) -> std::io::Result<()> {
    let Servers {
        conduit,
        postgres,
        host,
        router,
    } = servers;

    // TCP keepalive on the host socket: the relay is idle whenever the
    // plugin is, so a peer that died silently has to surface as a
    // socket error rather than an eternally quiet stream. The Postgres
    // listener needs none — its clients are in this same network
    // namespace, and their death is never silent.
    use axum::serve::ListenerExt;
    let host = host.tap_io(|io| objectiveai_sdk::net::set_tcp_keepalive(io));

    // Either listener failing takes the process down: half a conduit is
    // not a conduit.
    tokio::select! {
        result = accept_postgres(postgres, conduit) => result,
        result = axum::serve(host, router) => result,
    }
}

pub async fn run() -> std::io::Result<()> {
    let servers = setup().await?;
    tracing::info!(
        postgres = %servers.postgres.local_addr()?,
        host = %servers.host.local_addr()?,
        "listening",
    );
    serve(servers).await
}

/// Accept Postgres clients forever.
async fn accept_postgres(
    listener: tokio::net::TcpListener,
    conduit: Arc<Conduit>,
) -> std::io::Result<()> {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(client(stream, Arc::clone(&conduit)));
            }
            // Accept errors are per-connection (a descriptor limit, a
            // client that vanished during the handshake), not a broken
            // listener — dying here would take down a database that is
            // otherwise working. The pause is what keeps a persistent
            // error from becoming a hot spin.
            Err(error) => {
                tracing::warn!(%error, "accept failed");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// One Postgres client, from accept to close.
async fn client(stream: tokio::net::TcpStream, conduit: Arc<Conduit>) {
    // pgwire is a request/response conversation; Nagle would charge it
    // coalescing delay on every round trip.
    let _ = stream.set_nodelay(true);

    let Some(host) = conduit.host().await else {
        return;
    };

    let (mut read, mut write) = stream.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let id = conduit.open(host.epoch, tx);

    if host
        .tx
        .send(axum::extract::ws::Message::Binary(frame::encode_open(id)))
        .is_err()
    {
        conduit.close(id);
        return;
    }

    // The inbound pump. It ends when its sender is dropped — client
    // gone, host `Close`, or the relay detaching — and the FIN it
    // leaves behind is what tells the client its server went away.
    let writer = tokio::spawn(async move {
        while let Some(payload) = rx.recv().await {
            if write.write_all(&payload).await.is_err() {
                break;
            }
        }
        let _ = write.shutdown().await;
    });

    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        // Watching the relay alongside the client is what keeps this
        // connection from outliving the socket its far end depends on.
        // A client sitting on a half-finished round trip has nothing to
        // read and nothing to write, so without this it would wait on a
        // response no longer coming — and on loopback, with no keepalive
        // to notice, it would wait forever.
        let result = tokio::select! {
            _ = conduit.detached(host.epoch) => break,
            result = read.read(&mut buf) => result,
        };
        let filled = match result {
            Ok(0) | Err(_) => break,
            Ok(filled) => filled,
        };
        if host
            .tx
            .send(axum::extract::ws::Message::Binary(frame::encode_data(
                id,
                &buf[..filled],
            )))
            .is_err()
        {
            break;
        }
    }

    // Deregistering drops the pump's sender, which shuts the client
    // socket; the `Close` lets the host release the real connection
    // instead of leaving it attached to a stream nothing will ever read.
    conduit.close(id);
    let _ = host
        .tx
        .send(axum::extract::ws::Message::Binary(frame::encode_close(id)));
    // NOT awaited, deliberately. The pump has everything it needs to
    // finish on its own — drain, then shut the socket down — and waiting
    // would mean blocking on a client that has stopped reading, since a
    // full receive window parks `write_all` indefinitely. Nothing here
    // depends on that teardown having happened, and the stream is already
    // deregistered, so nothing can route to it either.
    drop(writer);
}

async fn upgrade(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::State(conduit): axum::extract::State<Arc<Conduit>>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| relay(socket, conduit))
}

/// Relay one host socket until it breaks.
async fn relay(socket: axum::extract::ws::WebSocket, conduit: Arc<Conduit>) {
    use axum::extract::ws::Message;
    use futures::{SinkExt as _, StreamExt as _};

    let (mut sink, mut stream) = socket.split();

    // ONE writer owns the sink. Every client's frames go through this
    // channel, so they interleave instead of queueing behind whichever
    // client got there first.
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });

    let epoch = conduit.attach(tx);

    while let Some(Ok(message)) = stream.next().await {
        match message {
            Message::Binary(bytes) => {
                if let Some(frame) = Frame::decode(bytes) {
                    conduit.route(frame);
                }
            }
            Message::Close(_) => break,
            // Text is not part of the format, and axum answers
            // ping/pong itself.
            _ => {}
        }
    }

    conduit.detach(epoch);
    writer.abort();
}
