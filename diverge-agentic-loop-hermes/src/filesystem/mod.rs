//! The filesystem under the Hermes home: what goes down before
//! `hermes gateway` starts, and what comes back up after it exits.
//!
//! Before the gateway starts, everything the request's agent says
//! has to be where Hermes reads it: credentials and switches in the
//! gateway's PROCESS ENVIRONMENT, selection and membership in
//! `config.yaml`, the OAuth state documents in the credential files
//! — `auth.json` entries for the state-carrying providers and
//! spotify, the Qwen CLI's token file at the real home, vertex's
//! service-account file — and, when the caller holds one, the
//! session's own state: `state.db` and the two memory files, which
//! is the [`continuation`]. After the gateway exits, what it changed
//! has to go back to the caller: every resource as the run left it,
//! then the continuation.
//!
//! Two entry points, one each way. [`prepare`] lays it all down in
//! one pass and hands back [`Prepared`]: the environment the spawner
//! sets on the gateway process, the API server key the run driver
//! will present, and the session to resume if one landed. [`finish`]
//! streams it all back up as [`Export`] items — the resources first,
//! the continuation last, since the closer closes. Between them,
//! [`history`] reads a session's transcript out of the landed
//! database for the run to resume from.
//!
//! # What is and is not here
//!
//! The request's own `environment` is the server's: it is applied
//! to the container itself before anything runs, and nothing here
//! reads or repeats it. What [`Prepared::env`] holds is the
//! HARNESS'S variables only, so on an overlap the harness wins by
//! construction. `system_prompt` and `effort` are per-request fields
//! of the run the driver POSTs, not filesystem. Mounts landed before
//! the container started.
//!
//! # Resources and the continuation, concurrently; files, once each
//!
//! A request may name several resources (one provider's OAuth state
//! and spotify's), each fetched from the caller over the container
//! surface, and the continuation is fetched the same way. All of it
//! is awaited TOGETHER — every resource ask goes out at once, and the
//! continuation's settlement (its chunks having landed on disk as
//! they came, its database checked) is awaited beside them — and
//! only then is anything written: each file is assembled whole from
//! what it needs and written exactly once, so two resources bound for
//! the same file (`auth.json`) never contend for it. The two halves
//! cannot contend either: resources land in `config.yaml`,
//! `auth.json` and the credential files; the continuation lands in
//! `state.db` and `memories/`.
//!
//! # `config.yaml` is JSON
//!
//! Hermes loads its config with a YAML parser, and JSON is YAML. The
//! document is rendered with serde_json and written under the name
//! Hermes looks for — no YAML dependency, no indentation to get
//! wrong.

mod ask;
mod config;
mod error;
mod export;
mod finish_error;
mod history;
mod history_error;
mod message;
mod plan;
mod prepared;
mod provider;
mod session;
mod toolsets;

pub mod continuation;

pub use ask::*;
pub use error::*;
pub use export::*;
pub use finish_error::*;
pub use history::*;
pub use history_error::*;
pub use message::*;
pub use plan::*;
pub use prepared::*;
pub use session::*;

use std::path::Path;

use diverge_provider_sdk::agentic_loop_container::request::Request;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::Agent;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::hermes;
use futures_util::{Stream, StreamExt as _, TryFutureExt as _};
use futures_util::future;
use uuid::Uuid;

use crate::fetcher::Fetcher;

/// Where Hermes keeps its state, fixed for the container's life: the
/// default home for the container's root user, pinned into the
/// gateway's environment as `HERMES_HOME` — so what this module lays
/// down is what the gateway finds, and what the gateway leaves is
/// what [`continuation::stream`] sweeps up.
pub const HERMES_HOME: &str = "/root/.hermes";

/// The name of the container's own MCP proxy in Hermes's server
/// list — the one server the agent's tool calls go to.
pub const MCP_PROXY_NAME: &str = "diverge";

/// Where that proxy serves: the Container specification's MCP port,
/// rmcp's conventional path, on loopback.
pub const MCP_PROXY_URL: &str = "http://127.0.0.1:8081/mcp";

/// Where the gateway's API server listens — loopback, for the run
/// driver beside it and nobody else.
pub const API_SERVER_HOST: &str = "127.0.0.1";

/// The API server's port: Hermes's own default, made explicit.
pub const API_SERVER_PORT: u16 = 8642;

/// The Qwen CLI's token file, at the REAL home: Hermes hardcodes
/// `Path.home()` for it and ignores its own `HERMES_HOME`.
pub const QWEN_CREDS: &str = "/root/.qwen/oauth_creds.json";

/// The session store, at the home's root — the continuation's, and
/// [`history`]'s to read.
const STATE_DB: &str = "state.db";

/// Hermes's configuration file, under the home.
const CONFIG_FILE: &str = "config.yaml";

/// Hermes's credential store, under the home.
const AUTH_FILE: &str = "auth.json";

/// Where vertex's service-account document is written, under the
/// home; `VERTEX_CREDENTIALS_PATH` names it.
const VERTEX_FILE: &str = "vertex-service-account.json";

/// Lay the filesystem down from the request and answer with what
/// the run still needs to carry.
///
/// The agent must be a `hermes` one ([`PrepareError::WrongAgent`]
/// otherwise — the caller has usually judged this already). Then,
/// in order: the provider's and the toolsets' contributions are
/// gathered into a [`Plan`]; the harness's own variables join them
/// (`HERMES_HOME`, `HERMES_YOLO_MODE`, the API server's
/// key/host/port — the key freshly generated, 64 hex characters, for
/// this run alone); every resource is fetched and parsed as a JSON
/// object WHILE the continuation is fetched and settled, the two
/// awaited together; and the files are written, each once:
/// `config.yaml`, `auth.json` when any entry needs it, the Qwen token
/// file, the vertex file. The continuation's own files are already
/// on disk by then — its chunks landed as they arrived.
///
/// Every ask reaches the socket through the driver, on the fetcher's
/// one channel. The fetcher is taken by value: this is where the
/// run's asking happens, and nothing asks after it.
pub async fn prepare(
    request: &Request,
    fetcher: Fetcher,
) -> Result<Prepared, PrepareError> {
    let mut plan = plan(request)?;
    // The geometry, pinned: the home the continuation and the
    // credential files were laid down under, whatever the
    // container's `HOME` says.
    plan.set("HERMES_HOME", HERMES_HOME.to_string())?;
    // The image restricts write_file/patch to its own data root;
    // empty lifts the restriction, and the workspace is the caller's
    // mounts, wherever they are.
    plan.set("HERMES_WRITE_SAFE_ROOT", String::new())?;
    plan.set("HERMES_YOLO_MODE", "1".to_string())?;
    let api_server_key =
        format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    plan.set("API_SERVER_KEY", api_server_key.clone())?;
    plan.set("API_SERVER_HOST", API_SERVER_HOST.to_string())?;
    plan.set("API_SERVER_PORT", API_SERVER_PORT.to_string())?;

    // Every resource at once, and the continuation beside them;
    // nothing is written until all are in.
    let fetcher = &fetcher;
    let documents = future::try_join_all(plan.asks.iter().map(|ask| async move {
        let text = fetcher
            .fetch_resource(ask.identity.clone())
            .await
            .map_err(|error| PrepareError::Resource {
                field: ask.field,
                error,
            })?;
        let document: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&text).map_err(|_| {
                PrepareError::ResourceNotObject { field: ask.field }
            })?;
        Ok::<_, PrepareError>((ask.target, document))
    }));
    let (documents, session) = future::try_join(
        documents,
        fetcher.fetch_continuation().map_err(PrepareError::Continuation),
    )
    .await?;

    // Then the files, each assembled whole and written once.
    let home = Path::new(HERMES_HOME);
    tokio::fs::create_dir_all(home).await?;
    let config = serde_json::to_vec_pretty(&config::render(&plan))?;
    tokio::fs::write(home.join(CONFIG_FILE), config).await?;

    let mut providers = serde_json::Map::new();
    for (target, document) in documents {
        match target {
            Target::AuthEntry(name) => {
                providers.insert(
                    name.to_string(),
                    serde_json::Value::Object(document),
                );
            }
            Target::QwenCreds => {
                let path = Path::new(QWEN_CREDS);
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(path, serde_json::to_vec_pretty(&document)?)
                    .await?;
            }
        }
    }
    if !providers.is_empty() {
        // Entries only: never `active_provider` (selection is
        // `model.provider` in the config), never
        // `suppressed_sources` (a store carrying it leaves the
        // provider inert).
        let store = serde_json::json!({
            "version": 1,
            "providers": providers,
        });
        tokio::fs::write(home.join(AUTH_FILE), serde_json::to_vec_pretty(&store)?)
            .await?;
    }
    if let Some(service_account) = &plan.vertex {
        tokio::fs::write(home.join(VERTEX_FILE), service_account).await?;
    }

    Ok(Prepared {
        session,
        env: plan.env,
        api_server_key,
    })
}

/// The request's agent, read into a [`Plan`]: what both entry points
/// start from. The wrong agent kind is refused here.
fn plan(request: &Request) -> Result<Plan, PrepareError> {
    let Agent::Hermes(agent) = &request.agent else {
        return Err(PrepareError::WrongAgent);
    };
    let agent: &hermes::Agent = agent;
    let mut plan = Plan::new(agent.model.clone());
    provider::apply(&agent.provider, &mut plan)?;
    toolsets::apply(&agent.toolsets, &mut plan)?;
    Ok(plan)
}

/// Stream the filesystem back out, after the gateway has exited:
/// every resource the request named, as the run left it, then the
/// continuation as the closer.
///
/// The request is read into the same [`Plan`] [`prepare`] built, for
/// the one thing it says here: which resources were named and which
/// file each landed in. Then, in that order, each is read back —
/// `auth.json` once, its `providers.<name>` entry re-serialized; the
/// Qwen file verbatim — and yielded whole under the request field's
/// dotted path. A resource that is no longer there yields nothing:
/// Hermes quarantines an entry whose refresh failed for good, and
/// the caller keeping what it had is the honest answer, not a lost
/// continuation. Last, the continuation: the database folded, then
/// the files streamed a piece at a time, one alive at once.
///
/// Every failure is an item, and ends the stream.
pub fn finish(request: &Request) -> impl Stream<Item = Result<Export, FinishError>> {
    let plan = plan(request);
    async_stream::try_stream! {
        let plan = plan?;
        let home = Path::new(HERMES_HOME);

        // The resources, as the run left them.
        let mut store: Option<serde_json::Value> = None;
        for ask in &plan.asks {
            let body = match ask.target {
                Target::AuthEntry(name) => {
                    if store.is_none() {
                        let bytes = tokio::fs::read(home.join(AUTH_FILE)).await?;
                        store = Some(serde_json::from_slice(&bytes)?);
                    }
                    let entry = store
                        .as_ref()
                        .and_then(|store| store.get("providers"))
                        .and_then(|providers| providers.get(name));
                    match entry {
                        Some(entry) => serde_json::to_vec(entry)?,
                        None => continue,
                    }
                }
                Target::QwenCreds => match tokio::fs::read(QWEN_CREDS).await {
                    Ok(bytes) => bytes,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        continue;
                    }
                    Err(error) => Err(FinishError::Io(error))?,
                },
            };
            yield Export::Resource {
                name: ask.field,
                body,
            };
        }

        // The continuation, last.
        let mut pieces = Box::pin(continuation::stream().await?);
        while let Some(piece) = pieces.next().await {
            yield Export::Continuation(piece?);
        }
    }
}
