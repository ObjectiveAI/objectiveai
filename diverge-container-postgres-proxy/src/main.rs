//! The Postgres conduit inside an agentic_loop container.
//!
//! To the agent beside it, a database: a plain TCP listener on the
//! container's loopback at port `14980`, which the agent's driver
//! dials as if it were Postgres. To the provider outside, a WebSocket
//! listener at `/` on port `14981`: the server connects in — one
//! connection at a time — and every connection the agent opens is
//! carried out over that socket, per the SDK's
//! [`postgres_proxy`](diverge_provider_sdk::postgres_proxy) wire, to
//! be spliced onto the caller's real database on the far side of the
//! provider protocol.
//!
//! Nothing is configured. Both ports are fixed, no environment is
//! read: this program is started inside an image someone else built,
//! and what that image's environment says is not addressed to it.

mod conduit;
mod connection;
mod ws;

use std::future::IntoFuture as _;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future;
use tokio::net::TcpListener;

/// The port the agent's driver dials: the container's loopback only,
/// since nothing outside the container has any business speaking
/// pgwire to it.
const POSTGRES_PORT: u16 = 14980;

/// The port the server connects into, serving the WebSocket at `/`.
const PORT: u16 = 14981;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(run());
}

async fn run() {
    let conduit = Arc::new(conduit::Conduit::new());

    let app = axum::Router::new()
        .route("/", axum::routing::any(ws::accept))
        .with_state(Arc::clone(&conduit));

    // Both listeners, or neither: a conduit with one face is not one.
    let (postgres, server) = future::try_join(
        TcpListener::bind(("127.0.0.1", POSTGRES_PORT)),
        TcpListener::bind(("0.0.0.0", PORT)),
    )
    .await
    .expect("a port could not be bound");

    // Either half ending ends the program: an accept loop that
    // stopped or a server that stopped are both a conduit that no
    // longer conducts.
    let accepting = pin!(accept(postgres, Arc::clone(&conduit)));
    let serving = pin!(axum::serve(server, app).into_future());
    match future::select(accepting, serving).await {
        future::Either::Left(((), _)) => {}
        future::Either::Right((result, _)) => {
            result.expect("the server stopped unexpectedly")
        }
    }
}

/// Accept the agent's connections, each onto its own task.
///
/// An accept error is that attempt's alone — a transient the kernel
/// reports, not the listener dying — so the loop pauses briefly and
/// goes on, rather than taking the conduit down with a socket it
/// never got.
async fn accept(listener: TcpListener, conduit: Arc<conduit::Conduit>) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(connection::serve(stream, Arc::clone(&conduit)));
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}
