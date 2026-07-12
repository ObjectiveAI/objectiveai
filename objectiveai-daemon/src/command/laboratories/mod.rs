//! `laboratories` — top-level CLI dispatch for laboratory containers
//! (podman containers the conduit dials as client-side MCP servers).
//! `spawn`/`kill` manage the machine's resident laboratory HOST (one
//! process per (machine, state), serving ALL of its laboratories over
//! `/laboratory` connections to every configured daemon); `config`
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

use crate::context::Context;
use crate::db::laboratory_attachments::Target;
use crate::error::Error;

pub mod attach;
pub mod config;
pub mod create;
pub mod delete;
pub mod detach;
pub mod kill;
pub mod list;
pub mod spawn;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Attach(req) => {
            let value = attach::execute(ctx, req).await?;
            once(Ok(ResponseItem::Attach(value)))
        }
        Request::AttachRequestSchema(req) => {
            let value = attach::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AttachRequestSchema(value)))
        }
        Request::AttachResponseSchema(req) => {
            let value = attach::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AttachResponseSchema(value)))
        }
        Request::Detach(req) => {
            let value = detach::execute(ctx, req).await?;
            once(Ok(ResponseItem::Detach(value)))
        }
        Request::DetachRequestSchema(req) => {
            let value = detach::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DetachRequestSchema(value)))
        }
        Request::DetachResponseSchema(req) => {
            let value = detach::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DetachResponseSchema(value)))
        }
        Request::Config(req) => {
            let inner = config::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Config)))
        }
        Request::Kill(req) => {
            let value = kill::execute(ctx, req).await?;
            once(Ok(ResponseItem::Kill(value)))
        }
        Request::KillRequestSchema(req) => {
            let value = kill::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::KillRequestSchema(value)))
        }
        Request::KillResponseSchema(req) => {
            let value = kill::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::KillResponseSchema(value)))
        }
        Request::Spawn(req) => {
            let value = spawn::execute(ctx, req).await?;
            once(Ok(ResponseItem::Spawn(value)))
        }
        Request::SpawnRequestSchema(req) => {
            let value = spawn::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SpawnRequestSchema(value)))
        }
        Request::SpawnResponseSchema(req) => {
            let value = spawn::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SpawnResponseSchema(value)))
        }
        Request::Create(req) => {
            let value = create::execute(ctx, req).await?;
            once(Ok(ResponseItem::Create(value)))
        }
        Request::CreateRequestSchema(req) => {
            let value = create::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::CreateRequestSchema(value)))
        }
        Request::CreateResponseSchema(req) => {
            let value = create::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::CreateResponseSchema(value)))
        }
        Request::Delete(req) => {
            let value = delete::execute(ctx, req).await?;
            once(Ok(ResponseItem::Delete(value)))
        }
        Request::DeleteRequestSchema(req) => {
            let value = delete::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DeleteRequestSchema(value)))
        }
        Request::DeleteResponseSchema(req) => {
            let value = delete::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DeleteResponseSchema(value)))
        }
        Request::List(req) => {
            let inner = list::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
    };
    Ok(stream)
}

/// Resolve `--machine` to the exact CONNECTED host `(machine id,
/// state)` that will serve the request, auto-spawning the local host
/// when the target is this machine and nothing is connected yet.
/// Shared by `create` (delete routes by laboratory id instead).
///
/// Hosts are one per (machine, state), and a machine may have SEVERAL
/// connected (different states). Preference order:
///
/// 1. `None` ⇒ the target machine is the current one (the daemon's
///    own machine id).
/// 2. The host matching this daemon's OWN state, when connected.
/// 3. Otherwise a UNIQUE connected host for the target machine —
///    whatever its state (the normal remote case: one host per
///    machine, its state name is its own business).
/// 4. Several candidates and none own-state ⇒ ambiguity error (naming
///    the states) rather than a silent pick.
/// 5. NONE connected: target is THIS machine ⇒ `laboratories spawn`
///    in-process (which errors when `laboratories config local` is
///    false — a local host that never dials this daemon can't serve
///    it) and wait for it to register; any other machine ⇒ error
///    (this daemon cannot spawn a host elsewhere).
pub(super) async fn resolve_host(
    ctx: &Context,
    machine: Option<String>,
) -> Result<(String, String), Error> {
    let local_machine = objectiveai_sdk::machine::machine_id(ctx.filesystem.dir());
    let own_state = ctx.filesystem.state().to_string();
    let target = machine.unwrap_or_else(|| local_machine.clone());
    let hubs = ctx.resident_hubs().ok_or_else(|| {
        Error::Laboratory("laboratories commands require the resident daemon".to_string())
    })?;
    if hubs.laboratories.has_host(&target, &own_state) {
        return Ok((target, own_state));
    }
    let mut hosts = hubs.laboratories.hosts_for_machine(&target);
    match hosts.len() {
        1 => {
            let (state, _) = hosts.remove(0);
            Ok((target, state))
        }
        0 => {
            if target == local_machine {
                // `spawn::spawn` waits for the local host to appear in
                // the registry (or fails fast), so the caller can
                // forward immediately after.
                spawn::spawn(ctx).await?;
                Ok((target, own_state))
            } else {
                Err(Error::Laboratory(format!(
                    "no laboratory host connected for machine '{target}' — run \
                     `laboratories spawn` on that machine with this daemon's address configured"
                )))
            }
        }
        _ => {
            let states: Vec<&str> =
                hosts.iter().map(|(state, _)| state.as_str()).collect();
            Err(Error::Laboratory(format!(
                "multiple laboratory hosts are connected for machine '{target}' \
                 (states: {}) and none matches this daemon's state '{own_state}' — \
                 the target is ambiguous",
                states.join(", ")
            )))
        }
    }
}

/// Best-effort local-host ensure for id-routed commands (`delete`,
/// `list`): spawn THIS machine's OWN-STATE host if it isn't connected.
/// Errors propagate (`delete` surfaces them; `list` drops them) —
/// including the `laboratories config local: false` refusal.
pub(super) async fn ensure_local_host(ctx: &Context) -> Result<(), Error> {
    let local_machine = objectiveai_sdk::machine::machine_id(ctx.filesystem.dir());
    let connected = ctx.resident_hubs().is_some_and(|hubs| {
        hubs.laboratories
            .has_host(&local_machine, ctx.filesystem.state())
    });
    if !connected {
        spawn::spawn(ctx).await?;
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
    ctx: &Context,
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
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            Ok(Target::Aih(format!("{parent}/{agent_instance}")))
        }
        AgentSelector::Tag { agent_tag } => {
            let pool = ctx.db_client().await?;
            match crate::db::tags::lookup(pool, agent_tag).await? {
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
