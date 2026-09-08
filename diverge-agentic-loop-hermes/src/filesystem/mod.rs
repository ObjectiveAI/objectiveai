//! The filesystem under the Hermes home: what goes down before
//! `hermes gateway` starts, and what is read back after it exits.
//!
//! Before the gateway starts, everything the request's agent says
//! has to be where Hermes reads it: credentials and switches in the
//! gateway's PROCESS ENVIRONMENT, selection and membership in
//! `config.yaml`, the OAuth state documents in the credential files
//! — `auth.json` entries for the state-carrying providers and
//! spotify, the Qwen CLI's token file at the real home, vertex's
//! service-account file. The session's own state — `state.db` and
//! the two memory files, the [`continuation`] — is the database's
//! business ([`crate::continuation`]), restored once for the
//! program's life. After the gateway exits, the rotating documents
//! are read back as the run left them, for the vault.
//!
//! Three entry points. [`plan()`] reads the agent into a [`Plan`]:
//! the environment, the config's inputs, and which vault documents
//! the run needs. [`prepare`] lays the plan down with those
//! documents in hand and hands back [`Prepared`]: the environment
//! the spawner sets on the gateway process and the API server key
//! the run driver will present. [`read_back`] reads each document
//! off disk after the gateway has exited. Between them,
//! [`history()`] reads a session's transcript out of the database
//! for the run to resume from, and [`session()`] its tip.
//!
//! # What is and is not here
//!
//! What [`Prepared::env`] holds is the HARNESS'S variables only, so
//! on an overlap with the container's own environment the harness
//! wins by construction. `system_prompt` and `effort` are
//! per-request fields of the run the driver POSTs, not filesystem.
//! Mounts landed before the container started; skills are whatever
//! is under [`EXTERNAL_SKILLS`] when the run begins.
//!
//! # `config.yaml` is JSON
//!
//! Hermes loads its config with a YAML parser, and JSON is YAML. The
//! document is rendered with serde_json and written under the name
//! Hermes looks for — no YAML dependency, no indentation to get
//! wrong.

mod config;
mod document;
mod error;
mod history;
mod history_error;
mod message;
mod plan;
mod prepared;
mod provider;
mod read_back_error;
mod session;
mod toolsets;

pub mod continuation;

pub use document::*;
pub use error::*;
pub use history::*;
pub use history_error::*;
pub use message::*;
pub use plan::*;
pub use prepared::*;
pub use read_back_error::*;
pub use session::*;

use std::path::Path;

use uuid::Uuid;

use crate::agent::Agent;
use crate::vault;

/// Where Hermes keeps its state, fixed for the container's life: the
/// default home for the container's root user, pinned into the
/// gateway's environment as `HERMES_HOME` — so what this module lays
/// down is what the gateway finds, and what the gateway leaves is
/// what [`continuation::stream`] sweeps up.
pub const HERMES_HOME: &str = "/root/.hermes";

/// The name of the proxy's MCP server in Hermes's server list — the
/// one server the agent's tool calls go to. Its URL is the container
/// SDK's to spell.
pub const MCP_PROXY_NAME: &str = "diverge";

/// Where the gateway's API server listens — loopback, for the run
/// driver beside it and nobody else.
pub const API_SERVER_HOST: &str = "127.0.0.1";

/// The API server's port: Hermes's own default, made explicit.
pub const API_SERVER_PORT: u16 = 8642;

/// The Qwen CLI's token file, at the REAL home: Hermes hardcodes
/// `Path.home()` for it and ignores its own `HERMES_HOME`.
pub const QWEN_CREDS: &str = "/root/.qwen/oauth_creds.json";

/// Where callers mount skills: the whole set here, or one skill per
/// `external-skills/<name>` directory, each holding its `SKILL.md`.
/// Named to Hermes as `skills.external_dirs` — directories it
/// discovers (recursively) and views but never writes, which is what
/// a read-only mount needs. Its own `skills/` beside this is off
/// limits for mounts: Hermes syncs its bundled skills into it at
/// startup and keeps the curator's, the usage tracker's and the
/// hub's bookkeeping there. A missing directory is silently
/// skipped, so the config always names it.
pub const EXTERNAL_SKILLS: &str = "/root/.hermes/external-skills";

/// The session store, at the home's root — the continuation's, and
/// [`history()`]'s to read.
const STATE_DB: &str = "state.db";

/// Hermes's configuration file, under the home.
const CONFIG_FILE: &str = "config.yaml";

/// Hermes's credential store, under the home.
const AUTH_FILE: &str = "auth.json";

/// Where vertex's service-account document is written, under the
/// home; `VERTEX_CREDENTIALS_PATH` names it.
const VERTEX_FILE: &str = "vertex-service-account.json";

/// The agent, read into a [`Plan`]: what every entry point starts
/// from. The provider's and the toolsets' contributions are
/// gathered, and the harness's own variables join them (`HERMES_HOME`,
/// `HERMES_YOLO_MODE`, the API server's key/host/port — the key
/// freshly generated, 64 hex characters, for this run alone).
pub async fn plan(agent: &Agent) -> Result<Plan, PrepareError> {
    let mut plan = Plan::new(agent.model.clone());
    provider::apply(&agent.provider, &mut plan)?;
    toolsets::apply(&agent.toolsets, skills_mounted().await?, &mut plan)?;
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
    plan.set("API_SERVER_KEY", api_server_key)?;
    plan.set("API_SERVER_HOST", API_SERVER_HOST.to_string())?;
    plan.set("API_SERVER_PORT", API_SERVER_PORT.to_string())?;
    Ok(plan)
}

/// Whether anything is mounted under [`EXTERNAL_SKILLS`] — a
/// directory of skills, one skill, or a lone `SKILL.md`. That is what
/// turns the `skills` toolset on. Mounts land before the container
/// starts, so the disk is the authority; a directory that is not
/// there is nothing mounted.
async fn skills_mounted() -> Result<bool, PrepareError> {
    match tokio::fs::read_dir(EXTERNAL_SKILLS).await {
        Ok(mut entries) => Ok(entries.next_entry().await?.is_some()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(PrepareError::Io(error)),
    }
}

/// Lay the plan down, with the vault documents in hand, and answer
/// with what the run still needs to carry.
///
/// The files are written, each once: `config.yaml`, `auth.json` when
/// any entry needs it, the Qwen token file, the vertex file. The
/// continuation's own files are the database's business and are not
/// touched here.
pub async fn prepare(
    plan: &Plan,
    documents: &[(&'static str, vault::Document)],
) -> Result<Prepared, PrepareError> {
    let home = Path::new(HERMES_HOME);
    tokio::fs::create_dir_all(home).await?;
    let config = serde_json::to_vec_pretty(&config::render(plan))?;
    tokio::fs::write(home.join(CONFIG_FILE), config).await?;

    let mut providers = serde_json::Map::new();
    for document in &plan.documents {
        let Some((_, content)) = documents.iter().find(|(key, _)| *key == document.key) else {
            continue;
        };
        match document.target {
            Target::AuthEntry(name) => {
                providers.insert(
                    name.to_string(),
                    serde_json::Value::Object(content.clone()),
                );
            }
            Target::QwenCreds => {
                let path = Path::new(QWEN_CREDS);
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(path, serde_json::to_vec_pretty(content)?).await?;
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
        env: plan.env.clone(),
        api_server_key: plan.env["API_SERVER_KEY"].clone(),
    })
}

/// Read every vault document back, after the gateway has exited, as
/// the run left it: `auth.json` once, its `providers.<name>` entry
/// re-serialized; the Qwen file verbatim. A document no longer there
/// is `None`: Hermes quarantines an entry whose refresh failed for
/// good, and the caller keeping what it had is the honest answer.
pub async fn read_back(
    plan: &Plan,
) -> Result<Vec<(&'static str, Option<Vec<u8>>)>, ReadBackError> {
    let home = Path::new(HERMES_HOME);
    let mut store: Option<serde_json::Value> = None;
    let mut documents = Vec::new();
    for document in &plan.documents {
        let body = match document.target {
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
                    Some(entry) => Some(serde_json::to_vec(entry)?),
                    None => None,
                }
            }
            Target::QwenCreds => match tokio::fs::read(QWEN_CREDS).await {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(ReadBackError::Io(error)),
            },
        };
        documents.push((document.key, body));
    }
    Ok(documents)
}
