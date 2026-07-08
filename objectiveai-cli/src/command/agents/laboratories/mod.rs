//! `agents laboratories` — CLI-side dispatch for attach/detach/list of
//! laboratory ids on an agent target (a tag or an instance hierarchy).

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::laboratories::{Request, ResponseItem};
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;

use crate::context::Context;
use crate::db::laboratory_attachments::Target;
use crate::error::Error;

pub mod attach;
pub mod detach;
pub mod list;

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
