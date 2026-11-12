//! `identity-diverge-network`, as a tool container: accounts at
//! Diverge's own issuer, kept, and identities signed in on the daemon
//! as them.
//!
//! `https://identity.diverge.network` is Diverge's OpenID Connect
//! provider. Every subject there is `<issuer>.<id>.<name>` — `github`
//! or `google`, a user's id there, and a name of that user's choosing
//! — and the **account** `<issuer>.<id>` owns every subject that
//! begins with it: see
//! [`identity_diverge_network`](diverge_sdk::identity_diverge_network).
//! The tool keeps accounts, each the issuer's own tokens, in its
//! database, and signs a subject in on the daemon's host with one: it
//! has the daemon begin the login, approves the code the daemon
//! answers as the account, and waits for the daemon to say it is
//! signed in. Nobody enters a code.
//!
//! The program is an HTTP server on the container's loopback, at the
//! port [`diverge_sdk::container_proxy::inside::port()`] names, and
//! the proxy beside it is its only caller: `POST /register` once,
//! `POST /secrets` and `POST /secrets/collect` around every activation
//! (the tool takes no secret), and the MCP server at `/mcp`, whose two
//! tools are in [`tools`]. `--schema`, the program's one argument,
//! prints the JSON Schema of the arguments it takes, which is the
//! `schema` of the image's `/DIVERGE.md` frontmatter.
//!
//! # Nothing of the proxy's before a tool is called
//!
//! The host injects the proxy at runtime, and it may not be up until
//! a request comes. So nothing here touches it at registration: the
//! database and the daemon are reached on the first tool call that
//! needs them, and kept.
//!
//! # No account can be added yet
//!
//! An account is signed in at its issuer, by a person, and the tool's
//! page for that is not built. Until it is, the tool holds none.

mod arguments;
mod daemon;
mod failure;
mod permissions;
mod register;
mod secrets;
mod server;
mod store;
mod tools;

use std::io::Write as _;
use std::sync::Arc;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};

use crate::arguments::Arguments;
use crate::server::Server;

/// The program: the server, or — on `--schema` — the schema.
///
/// `--schema` alone prints the JSON Schema of [`Arguments`] on stdout
/// and exits 0; no argument is the server, for the container's life;
/// anything else is a usage line on stderr and exit 2. The image's
/// entrypoint never passes the argument: it exists so `/DIVERGE.md`
/// is written from the type the registration reads.
fn main() {
    let mut arguments = std::env::args_os().skip(1);
    match (arguments.next(), arguments.next()) {
        (None, None) => {}
        (Some(argument), None) if argument == "--schema" => return schema(),
        _ => {
            let _ = writeln!(std::io::stderr(), "usage: identity-diverge-network [--schema]");
            std::process::exit(2);
        }
    }
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("the runtime could not be built");
    runtime.block_on(serve());
}

/// `--schema`: the JSON Schema of [`Arguments`], pretty-printed on
/// stdout with a trailing newline; a failure to write it is said on
/// stderr and exit 1, never a partial schema passed off as the answer.
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

/// The server, for the container's life, bound on the loopback: the
/// proxy is the only thing that dials it, and it is beside it. Every
/// MCP session the proxy opens shares one [`Server`], so the database
/// pool and the daemon connection are made once.
async fn serve() {
    let server = Server::new();
    let mcp = StreamableHttpService::new(move || Ok(server.clone()), Arc::new(LocalSessionManager::default()), StreamableHttpServerConfig::default());
    let app = axum::Router::new()
        .route("/register", axum::routing::post(register::register))
        .route("/secrets", axum::routing::post(secrets::feed))
        .route("/secrets/collect", axum::routing::post(secrets::collect))
        .nest_service("/mcp", mcp);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", diverge_sdk::container_proxy::inside::port())).await.expect("the port could not be bound");
    axum::serve(listener, app).await.expect("the server stopped unexpectedly");
}
