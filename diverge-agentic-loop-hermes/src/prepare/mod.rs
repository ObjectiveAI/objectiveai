//! The pre-gateway filesystem, rendered from the request.
//!
//! Before `hermes gateway` starts, everything the request's agent
//! says has to be where Hermes reads it: credentials and switches in
//! the gateway's PROCESS ENVIRONMENT, selection and membership in
//! `config.yaml`, and the OAuth state documents in the credential
//! files — `auth.json` entries for the state-carrying providers and
//! spotify, the Qwen CLI's token file at the real home, vertex's
//! service-account file. [`prepare`] does all of it in one pass and
//! hands back [`Prepared`]: the environment the spawner sets on the
//! gateway process, and the API server key the run driver will
//! present.
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
//! # Resources, concurrently; files, once each
//!
//! A request may name several resources (one provider's OAuth state
//! and spotify's), each fetched from the caller over the container
//! surface. They are fetched TOGETHER — every ask goes out at once
//! and all are awaited — and only then is anything written: each
//! file is assembled whole from what it needs and written exactly
//! once, so two resources bound for the same file (`auth.json`)
//! never contend for it.
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
mod plan;
mod prepared;
mod provider;
mod toolsets;

pub use ask::*;
pub use error::*;
pub use plan::*;
pub use prepared::*;

use std::path::Path;

use diverge_provider_sdk::agentic_loop_container::request::Request;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::Agent;
use futures_util::future;
use uuid::Uuid;

use crate::continuation::HERMES_HOME;
use crate::resource_fetcher::ResourceFetcher;

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

/// Hermes's configuration file, under the home.
const CONFIG_FILE: &str = "config.yaml";

/// Hermes's credential store, under the home.
const AUTH_FILE: &str = "auth.json";

/// Where vertex's service-account document is written, under the
/// home; `VERTEX_CREDENTIALS_PATH` names it.
const VERTEX_FILE: &str = "vertex-service-account.json";

/// Render the request's agent onto the filesystem and answer with
/// the gateway's environment.
///
/// The agent must be a `hermes` one ([`PrepareError::WrongAgent`]
/// otherwise — the caller has usually judged this already). Then,
/// in order: the provider's and the toolsets' contributions are
/// gathered into a [`Plan`]; the harness's own variables join them
/// (`HERMES_YOLO_MODE`, the API server's key/host/port — the key
/// freshly generated, 64 hex characters, for this run alone); every
/// resource is fetched at once and parsed as a JSON object; and the
/// files are written, each once: `config.yaml`, `auth.json` when any
/// entry needs it, the Qwen token file, the vertex file.
pub async fn prepare(
    request: &Request,
    fetcher: &ResourceFetcher,
) -> Result<Prepared, PrepareError> {
    let Agent::Hermes(agent) = &request.agent else {
        return Err(PrepareError::WrongAgent);
    };

    let mut plan = Plan::new(agent.model.clone());
    provider::apply(&agent.provider, &mut plan)?;
    toolsets::apply(&agent.toolsets, &mut plan)?;
    plan.set("HERMES_YOLO_MODE", "1".to_string())?;
    let api_server_key =
        format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    plan.set("API_SERVER_KEY", api_server_key.clone())?;
    plan.set("API_SERVER_HOST", API_SERVER_HOST.to_string())?;
    plan.set("API_SERVER_PORT", API_SERVER_PORT.to_string())?;

    // Every resource at once; nothing is written until all are in.
    let documents = future::try_join_all(plan.asks.iter().map(|ask| async move {
        let text = fetcher
            .fetch(ask.identity.clone())
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
    }))
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
        env: plan.env,
        api_server_key,
    })
}
