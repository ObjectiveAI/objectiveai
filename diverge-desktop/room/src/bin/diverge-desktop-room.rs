//! The room program, as a tool container runs it: an HTTP server on the
//! container's loopback, at the port the proxy dials.

use std::sync::Arc;

use diverge_desktop_room::image::{ProxyHost, router, Program};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let program = Program::new(Arc::new(ProxyHost::new()));
    let port = diverge_sdk::container_proxy::inside::port();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.expect("the loopback port is free");
    axum::serve(listener, router(program)).await.expect("the room program serves until the container stops");
}
