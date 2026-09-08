//! The proxy inside a Diverge container.
//!
//! One program beside every container's own entrypoint, serving the
//! SDK's [`container_proxy`]
//! wire on port `14979`: the container's asks go out on `/requests`,
//! the server answers each on the ask's own path, and the paths the
//! server opens on its own — the filetree, a file read, a file write
//! — are served here too.
//!
//! Built one feature at a time. Today: MCP, the vault, commands,
//! Postgres and the filetree. To the agent beside it the proxy is a
//! fully compliant MCP server at `/mcp/agent`; every exchange the
//! agent asks of it becomes an ask on `/requests`, answered on
//! `/mcp/list-tools/{channel}` and its siblings by the caller's own
//! servers on the far side of the provider. The vault is plain HTTP
//! at `/vault/agent/<op>`, each call one ask, answered on
//! `/vault/<op>/{channel}`; a command is `POST /command/agent`, its
//! items streamed back as they land from `/command/{channel}`. The
//! proxy is a database: a pgwire listener on the loopback at `14980`,
//! each connection the driver opens announced as one ask and carried,
//! raw, on `/postgres/{channel}`. And it is the caller's window: every
//! `/filetree` the server opens gets the container's filesystem,
//! watched from `/`, as a snapshot and then its changes; every
//! `/read` one file out of it, its bytes then the close; and every
//! `/write` one file into it, moved into place whole. And for an
//! agent container the proxy is the loop's face: `/run-loop` hands
//! the request to the harness attached at `/run-loop/agent` and
//! relays its chunks back, and `/agent-schema` answers the schema the
//! harness posted at `/agent-schema/agent`.

mod agent_schema;
mod command;
mod filetree;
mod mcp;
mod paths;
mod postgres;
mod read;
mod requests;
mod run_loop;
mod state;
mod vault;
mod write;
mod ws;

use std::sync::Arc;

use diverge_provider_sdk::container_proxy;
use futures_util::future;
use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use tokio::net::TcpListener;

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the runtime could not be built");
    runtime.block_on(run());
}

async fn run() {
    let requests = Arc::new(requests::Requests::new());
    let peers = Arc::new(mcp::Peers::new());
    let gate = Arc::new(mcp::Gate::new());
    // The server's mounts, or nothing: an unset or unreadable
    // variable is the empty set, by the SDK's rule.
    let ignore = Arc::new(filetree::Ignore::new(
        container_proxy::filetree::Ignore::parse(
            &std::env::var(container_proxy::filetree::IGNORE_ENV)
                .unwrap_or_default(),
        ),
    ));

    let run_loop = Arc::new(run_loop::RunLoop::new());
    let agent_schema = Arc::new(agent_schema::AgentSchema::new());

    tokio::spawn(mcp::notifications(
        Arc::clone(&requests),
        Arc::clone(&peers),
        Arc::clone(&gate),
    ));

    let agent = StreamableHttpService::new(
        {
            let requests = Arc::clone(&requests);
            let peers = Arc::clone(&peers);
            let gate = Arc::clone(&gate);
            move || {
                Ok(mcp::Handler {
                    requests: Arc::clone(&requests),
                    peers: Arc::clone(&peers),
                    gate: Arc::clone(&gate),
                })
            }
        },
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    let app = axum::Router::new()
        .route("/requests", axum::routing::any(ws::requests))
        .route(
            "/mcp/list-tools/{channel}",
            axum::routing::any(ws::mcp_list_tools),
        )
        .route(
            "/mcp/list-resources/{channel}",
            axum::routing::any(ws::mcp_list_resources),
        )
        .route(
            "/mcp/call-tool/{channel}",
            axum::routing::any(ws::mcp_call_tool),
        )
        .route(
            "/mcp/read-resource/{channel}",
            axum::routing::any(ws::mcp_read_resource),
        )
        .route(
            "/mcp/notifications/{channel}",
            axum::routing::any(ws::mcp_notifications),
        )
        .route("/vault/get/{channel}", axum::routing::any(ws::vault_get))
        .route("/vault/set/{channel}", axum::routing::any(ws::vault_set))
        .route(
            "/vault/delete/{channel}",
            axum::routing::any(ws::vault_delete),
        )
        .route("/vault/lock/{channel}", axum::routing::any(ws::vault_lock))
        .route(
            "/vault/unlock/{channel}",
            axum::routing::any(ws::vault_unlock),
        )
        .route("/vault/agent/get", axum::routing::post(vault::get))
        .route("/vault/agent/set", axum::routing::post(vault::set))
        .route("/vault/agent/delete", axum::routing::post(vault::delete))
        .route("/vault/agent/lock", axum::routing::post(vault::lock))
        .route("/vault/agent/unlock", axum::routing::post(vault::unlock))
        .route("/command/{channel}", axum::routing::any(ws::command))
        .route("/command/agent", axum::routing::post(command::agent))
        .route("/postgres/{channel}", axum::routing::any(ws::postgres))
        .route("/filetree", axum::routing::any(ws::filetree))
        .route("/read", axum::routing::any(ws::read))
        .route("/write", axum::routing::any(ws::write))
        .route("/run-loop", axum::routing::any(run_loop::server))
        .route("/run-loop/agent", axum::routing::any(run_loop::harness))
        .route("/agent-schema", axum::routing::any(agent_schema::server))
        .route("/agent-schema/agent", axum::routing::post(agent_schema::agent))
        .nest_service("/mcp/agent", agent)
        .with_state(state::AppState {
            requests: Arc::clone(&requests),
            ignore,
            run_loop,
            agent_schema,
        });

    // Both listeners or neither: a proxy that could answer asks but
    // not take the driver's connections would be a database that is
    // sometimes there.
    let (listener, loopback) = future::try_join(
        TcpListener::bind(("0.0.0.0", container_proxy::PORT)),
        TcpListener::bind((
            "127.0.0.1",
            container_proxy::postgres::LOOPBACK_PORT,
        )),
    )
    .await
    .expect("a port could not be bound");

    tokio::spawn(postgres::accept(loopback, Arc::clone(&requests)));

    axum::serve(listener, app)
        .await
        .expect("the server stopped unexpectedly");
}
