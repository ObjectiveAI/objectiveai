//! `laboratories` — top-level CLI dispatch for laboratory containers
//! (podman containers the conduit dials as client-side MCP servers).
//! The machine's resident laboratory HOST (one process per (machine,
//! state), serving ALL of its laboratories over `/laboratory`
//! connections to every configured daemon) is spawned implicitly by
//! the flows that need it (`ensure_host`/`ensure_local_host`) and
//! dies with the daemon — no spawn/kill commands; `config`
//! holds its dial list (`addresses`, each with an optional signature)
//! and the `local` toggle. `create`/`delete` forward over the owning
//! host's WS — podman runs host-side, wherever that is. `list` streams
//! the daemon's registry (hosts announce + notify; nothing scans).
//! `attach`/`detach` record/remove a laboratory id on an agent target
//! (a tag, or an instance hierarchy) in the CLI's database — read
//! attachments back via `agents instances get` (the `laboratories`
//! field).

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::laboratories::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::db::laboratory_attachments::Target;
use crate::error::Error;

pub mod attach;
pub mod config;
pub mod create;
pub mod delete;
pub mod detach;
pub mod list;
pub mod spawn;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Attach(req) => {
            let value = attach::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Attach(value)))
        }
        Request::AttachRequestSchema(req) => {
            let value = attach::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::AttachRequestSchema(value)))
        }
        Request::AttachResponseSchema(req) => {
            let value = attach::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::AttachResponseSchema(value)))
        }
        Request::Detach(req) => {
            let value = detach::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Detach(value)))
        }
        Request::DetachRequestSchema(req) => {
            let value = detach::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::DetachRequestSchema(value)))
        }
        Request::DetachResponseSchema(req) => {
            let value = detach::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::DetachResponseSchema(value)))
        }
        Request::Config(req) => {
            let inner = config::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Config)))
        }
        Request::Create(req) => {
            let value = create::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Create(value)))
        }
        Request::CreateRequestSchema(req) => {
            let value = create::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::CreateRequestSchema(value)))
        }
        Request::CreateResponseSchema(req) => {
            let value = create::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::CreateResponseSchema(value)))
        }
        Request::Delete(req) => {
            let value = delete::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Delete(value)))
        }
        Request::DeleteRequestSchema(req) => {
            let value = delete::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::DeleteRequestSchema(value)))
        }
        Request::DeleteResponseSchema(req) => {
            let value = delete::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::DeleteResponseSchema(value)))
        }
        Request::List(req) => {
            let inner = list::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
    };
    Ok(stream)
}

/// Resolve a command's optional `--machine`/`--machine-state` pair to
/// the exact laboratory-host identity it addresses — NAIVE, no
/// preference logic:
///
/// - Both given ⇒ that pair, verbatim.
/// - Neither ⇒ auto-fill (the current machine's id, the daemon's own
///   state).
/// - Exactly one ⇒ error (they travel together).
pub(super) fn resolve_pair(
    _global: &GlobalContext, scoped: &ScopedContext,
    machine: Option<String>,
    machine_state: Option<String>,
) -> Result<(String, String), Error> {
    match (machine, machine_state) {
        (Some(machine), Some(machine_state)) => Ok((machine, machine_state)),
        (None, None) => Ok((
            objectiveai_sdk::machine::machine_id(scoped.filesystem.dir()),
            scoped.filesystem.state().to_string(),
        )),
        _ => Err(Error::Laboratory(
            "machine and machine_state must be provided together".to_string(),
        )),
    }
}

/// Ensure a CONNECTED host for the exact `(machine id, state)` pair,
/// auto-spawning when the pair IS this daemon's own (local machine +
/// own state — the spawn errors when `laboratories config local` is
/// false, and waits for the host to register otherwise).
/// Any other unconnected pair is an error: this daemon cannot spawn a
/// host elsewhere (nor for another state).
pub(super) async fn ensure_host(
    global: &GlobalContext, scoped: &ScopedContext,
    machine: &str,
    machine_state: &str,
) -> Result<(), Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Laboratory("laboratories commands require the resident daemon".to_string())
    })?;
    if hubs.laboratories.has_host(machine, machine_state) {
        return Ok(());
    }
    let local_machine = objectiveai_sdk::machine::machine_id(scoped.filesystem.dir());
    if machine == local_machine && machine_state == scoped.filesystem.state() {
        // `spawn::spawn` waits for the local host to appear in the
        // registry (or fails fast), so the caller can forward
        // immediately after.
        spawn::spawn(global, scoped).await?;
        Ok(())
    } else {
        Err(Error::Laboratory(format!(
            "no laboratory host connected for machine '{machine}' state \
             '{machine_state}' — run any laboratories command on that \
             machine/state (its daemon auto-spawns the host) with this \
             daemon's address configured"
        )))
    }
}

/// Best-effort local-host ensure for id-routed commands (`delete`,
/// `list`): spawn THIS machine's OWN-STATE host if it isn't connected.
/// Errors propagate (`delete` surfaces them; `list` drops them) —
/// including the `laboratories config local: false` refusal.
pub(crate) async fn ensure_local_host(global: &GlobalContext, scoped: &ScopedContext) -> Result<(), Error> {
    let local_machine = objectiveai_sdk::machine::machine_id(scoped.filesystem.dir());
    let connected = global.resident_hubs().is_some_and(|hubs| {
        hubs.laboratories
            .has_host(&local_machine, scoped.filesystem.state())
    });
    if !connected {
        spawn::spawn(global, scoped).await?;
    }
    Ok(())
}

/// Resolve the agent target to its DB key. Shared by `attach` +
/// `detach`.
///
/// NO LOCKING — attachments may be changed at ANY time, active agents
/// included. A change never affects an agent mid-completion: the spawn
/// re-resolves attachments at every restart-pass boundary (each pass
/// dials whatever is attached NOW), so the change takes shape once the
/// agent finishes its current pass and wakes/respawns.
///
/// - **Instance** (PAIH + `--agent-instance`) → keyed on the AIH.
/// - **Tag** (GROUPED or BOUND) → keyed on the tag, which must exist.
/// - **Ref** (a direct agent spec) → error (no tag/AIH to key on).
pub(super) async fn resolve_target(
    global: &GlobalContext, scoped: &ScopedContext,
    selector: &AgentSelector,
) -> Result<Target, Error> {
    match selector {
        AgentSelector::Ref { .. } => Err(Error::LaboratoryRefTarget),
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(scoped.agent_instance_hierarchy());
            Ok(Target::Aih(format!("{parent}/{agent_instance}")))
        }
        AgentSelector::Tag { agent_tag } => {
            let pool = global.db_client().await?;
            match crate::db::tags::lookup(&pool, agent_tag).await? {
                crate::db::tags::LookupState::Absent => {
                    Err(Error::TagNotFound(agent_tag.clone()))
                }
                crate::db::tags::LookupState::Grouped { .. }
                | crate::db::tags::LookupState::Bound { .. } => {
                    Ok(Target::Tag(agent_tag.clone()))
                }
            }
        }
    }
}
