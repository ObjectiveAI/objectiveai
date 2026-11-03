//! `human`, as an agent container: an agent with no model, whose every
//! word and every tool call is a person's, given on its frontend.
//!
//! A run comes in as any agent's does: the proxy posts the messages to
//! `/run`. Nothing answers them until the person does: the frontend
//! shows them, and the person writes the agent's replies, calls the
//! agent's tools by hand, and ends the turn, which ends the run. A
//! message that arrives while the person is at it is shown at once,
//! and is part of the same turn.
//!
//! The program is an HTTP server on the container's loopback, at the
//! port [`diverge_sdk::container_proxy::inside::port()`] names, and the
//! proxy beside it is its only caller: `POST /register` once,
//! `POST /secrets` and `POST /secrets/collect` around every activation
//! (the agent takes no secret), `POST /run`, `POST /enqueue` and
//! `POST /dequeue`, which are in [`run`], and the frontend beneath
//! `/frontend`, in [`frontend`]. `--schema`, the program's one
//! argument, prints the JSON Schema of the arguments it takes, which is
//! the `schema` of the image's `/DIVERGE.md` frontmatter.
//!
//! # Nothing of the proxy's before it is needed
//!
//! The host injects the proxy at runtime, and it may not be up until a
//! request comes. So nothing here touches it at registration: the
//! database and the MCP session are reached on first use, and kept.

mod arguments;
mod caller;
mod desk;
mod frontend;
mod register;
mod run;
mod secrets;
mod store;

use std::io::Write as _;

use crate::arguments::Arguments;
use crate::desk::Desk;

/// The program: the server, or — on `--schema` — the schema.
///
/// `--schema` alone prints the JSON Schema of [`Arguments`] on stdout
/// and exits 0; no argument is the server, for the container's life;
/// anything else is a usage line on stderr and exit 2.
fn main() {
    let mut arguments = std::env::args_os().skip(1);
    match (arguments.next(), arguments.next()) {
        (None, None) => {}
        (Some(argument), None) if argument == "--schema" => return schema(),
        _ => {
            let _ = writeln!(std::io::stderr(), "usage: human [--schema]");
            std::process::exit(2);
        }
    }
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("the runtime could not be built");
    runtime.block_on(serve());
}

/// `--schema`: the JSON Schema of [`Arguments`], pretty-printed on
/// stdout with a trailing newline.
fn schema() {
    let text = match serde_json::to_string_pretty(&schemars::schema_for!(Arguments)) {
        Ok(text) => text,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = writeln!(std::io::stdout(), "{text}") {
        let _ = writeln!(std::io::stderr(), "{error}");
        std::process::exit(1);
    }
}

/// The server, for the container's life, bound on the loopback. The
/// run and every open page share one [`Desk`].
async fn serve() {
    let desk = Desk::new();
    let app = axum::Router::new()
        .route("/register", axum::routing::post(register::register))
        .route("/secrets", axum::routing::post(secrets::feed))
        .route("/secrets/collect", axum::routing::post(secrets::collect))
        .route("/run", axum::routing::post(run::run))
        .route("/enqueue", axum::routing::post(run::enqueue))
        .route("/dequeue", axum::routing::post(run::dequeue))
        .with_state(desk.clone())
        .merge(frontend::routes(desk));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", diverge_sdk::container_proxy::inside::port())).await.expect("the port could not be bound");
    axum::serve(listener, app).await.expect("the server stopped unexpectedly");
}
