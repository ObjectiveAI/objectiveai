//! `tasks` — CLI-side dispatch for the tasks subtree.
//! Three leaves today: `schedule`, `list`, `run`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::tasks::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod list;
pub mod run;
pub mod schedule;

/// Resolve the scope `(agent_instance_hierarchy?, tag?)` pair
/// shared by `list` and `run` into a single hierarchy string.
/// `tag` resolves BOUND-only — GROUPED / ABSENT raise structured
/// errors. When neither is given, falls back to the cli's own
/// `Config.agent_instance_hierarchy`.
pub(crate) async fn resolve_scope(
    ctx: &Context,
    agent_instance_hierarchy: Option<String>,
    tag: Option<String>,
) -> Result<String, Error> {
    match (agent_instance_hierarchy, tag) {
        (Some(h), None) => Ok(h),
        (None, Some(tag)) => {
            use crate::db::tags;
            match tags::lookup(&ctx.db, &tag).await? {
                tags::LookupState::Bound { agent_instance_hierarchy } => {
                    Ok(agent_instance_hierarchy)
                }
                tags::LookupState::Grouped {
                    tag_group_id,
                    parent_agent_instance_hierarchy,
                    ..
                } => Err(Error::TagGrouped {
                    tag,
                    tag_group_id,
                    parent_agent_instance_hierarchy,
                }),
                tags::LookupState::Absent => Err(Error::TagNotFound(tag)),
            }
        }
        (None, None) => Ok(ctx.config.agent_instance_hierarchy.clone()),
        (Some(_), Some(_)) => unreachable!(
            "clap group `scope` enforces mutex between --agent-instance-hierarchy / --tag"
        ),
    }
}

/// Resolve one `list` `Target` to the AIH it designates. `Me` → the
/// cli's own hierarchy; `Direct` → `parent.unwrap_or(default)/leaf`;
/// `Tag` → BOUND lookup (GROUPED / ABSENT raise structured errors,
/// same posture as [`resolve_scope`]).
pub(crate) async fn resolve_target(
    db: &crate::db::Pool,
    target: objectiveai_sdk::cli::command::tasks::list::Target,
    default_parent: &str,
) -> Result<String, Error> {
    use crate::db::tags;
    use objectiveai_sdk::cli::command::tasks::list::Target;
    match target {
        Target::Me => Ok(default_parent.to_string()),
        Target::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| default_parent.to_string());
            Ok(format!("{parent}/{agent_instance}"))
        }
        Target::Tag { agent_tag } => match tags::lookup(db, &agent_tag).await? {
            tags::LookupState::Bound {
                agent_instance_hierarchy,
            } => Ok(agent_instance_hierarchy),
            tags::LookupState::Grouped {
                tag_group_id,
                parent_agent_instance_hierarchy,
                ..
            } => Err(Error::TagGrouped {
                tag: agent_tag,
                tag_group_id,
                parent_agent_instance_hierarchy,
            }),
            tags::LookupState::Absent => Err(Error::TagNotFound(agent_tag)),
        },
    }
}

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Schedule(req) => {
            let value = schedule::execute(ctx, req).await?;
            once(Ok(ResponseItem::Schedule(value)))
        }
        Request::ScheduleRequestSchema(req) => {
            let value = schedule::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ScheduleRequestSchema(value)))
        }
        Request::ScheduleResponseSchema(req) => {
            let value = schedule::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ScheduleResponseSchema(value)))
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
        Request::Run(req) => {
            let inner = run::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Run)))
        }
        Request::RunRequestSchema(req) => {
            let value = run::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RunRequestSchema(value)))
        }
        Request::RunResponseSchema(req) => {
            let value = run::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RunResponseSchema(value)))
        }
    };
    Ok(stream)
}
