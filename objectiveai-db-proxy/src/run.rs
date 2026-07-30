//! Config and the two listeners.
//!
//! Mirrors the `objectiveai-mcp-laboratory` `run.rs` shape so other
//! crates can `use objectiveai_db_proxy::{ConfigBuilder, run}` and
//! spawn the proxy in-process without going through the binary.
//!
//! There are two sockets, and which way each one faces is the point:
//!
//! | role             | default bind         | protocol  |
//! |------------------|----------------------|-----------|
//! | Postgres clients | `127.0.0.1:14979`    | raw TCP   |
//! | laboratory host  | `0.0.0.0:14980`      | WebSocket |
//!
//! The Postgres side is loopback because nothing outside the container
//! has any business reaching it, and its port is FIXED because that is
//! what lets a connection string be stamped into container env before
//! this process exists — which is the whole reason a plugin can stay
//! ignorant of the mechanism. Inside a container's own network
//! namespace there is no contention to randomize away from.
//!
//! The host side binds `0.0.0.0` because a published container port
//! does not reach a loopback-bound listener.
//!
//! Neither default collides with anything already in these containers:
//! `14978` is the laboratory MCP server's port.

use std::sync::Arc;

use envconfig::Envconfig;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::mpsc;

use crate::conduit::Conduit;
use crate::frame::{self, Frame};

/// How much is read from a client socket at a time.
///
/// A read size, NOT a protocol limit: a Postgres message larger than
/// this simply spans several frames, because both ends of the conduit
/// reassemble from a byte stream exactly as they would from a socket.
const READ_CHUNK: usize = 16 * 1024;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "POSTGRES_ADDRESS")]
    postgres_address: Option<String>,
    #[envconfig(from = "POSTGRES_PORT")]
    postgres_port: Option<u16>,
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "SUPPRESS_OUTPUT")]
    suppress_output: Option<String>,
}

impl EnvConfigBuilder {
    fn build(self) -> ConfigBuilder {
        ConfigBuilder {
            postgres_address: self.postgres_address,
            postgres_port: self.postgres_port,
            address: self.address,
            port: self.port,
            suppress_output: self.suppress_output.map(|v| {
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
            }),
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub postgres_address: Option<String>,
    pub postgres_port: Option<u16>,
    pub address: Option<String>,
    pub port: Option<u16>,
    pub suppress_output: Option<bool>,
}

impl Envconfig for ConfigBuilder {
    #[allow(deprecated)]
    fn init() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init().map(|e| e.build())
    }

    fn init_from_env() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_env().map(|e| e.build())
    }

    fn init_from_hashmap(
        hashmap: &std::collections::HashMap<String, String>,
    ) -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_hashmap(hashmap).map(|e| e.build())
    }
}

impl ConfigBuilder {
    pub fn build(self) -> Config {
        Config {
            postgres_address: self
                .postgres_address
                .unwrap_or_else(|| "127.0.0.1".to_string()),
            postgres_port: self.postgres_port.unwrap_or(14979),
            address: self.address.unwrap_or_else(|| "0.0.0.0".to_string()),
            port: self.port.unwrap_or(14980),
            suppress_output: self.suppress_output.unwrap_or(false),
        }
    }
}

pub struct Config {
    /// Where Postgres clients connect (env `POSTGRES_ADDRESS`).
    pub postgres_address: String,
    /// The port Postgres clients connect to (env `POSTGRES_PORT`). The
    /// launcher stamps a connection string naming this port into the
    /// container's environment, so it has to be known in advance.
    pub postgres_port: u16,
    /// Where the laboratory host dials in (env `ADDRESS`).
    pub address: String,
    /// The port the laboratory host dials (env `PORT`), published out
    /// of the container by the launcher.
    pub port: u16,
    pub suppress_output: bool,
}

pub struct Servers {
    pub conduit: Arc<Conduit>,
    pub postgres: tokio::net::TcpListener,
    pub host: tokio::net::TcpListener,
    pub router: axum::Router,
}

pub async fn setup(config: Config) -> std::io::Result<Servers> {
    let Config {
        postgres_address,
        postgres_port,
        address,
        port,
        suppress_output: _,
    } = config;

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
        tokio::net::TcpListener::bind(format!("{postgres_address}:{postgres_port}")),
        tokio::net::TcpListener::bind(format!("{address}:{port}")),
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

pub async fn run(config: Config) -> std::io::Result<()> {
    let suppress_output = config.suppress_output;
    let servers = setup(config).await?;
    if !suppress_output {
        tracing::info!(
            postgres = %servers.postgres.local_addr()?,
            host = %servers.host.local_addr()?,
            "listening",
        );
    }
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
    let _ = writer.await;
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
