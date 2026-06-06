//! `agents tasks` — CLI-side dispatch for the tasks subtree.
//! Three leaves today: `schedule`, `list`, `run`.

use std::pin::Pin;

use futures::{Stream, StreamExt, stream};
use objectiveai_sdk::cli::command::agents::tasks::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod list;
pub mod run;
pub mod schedule;

/// Resolve the scope `(agent_instance_hierarchy?, tag?)` pair
/// shared by `list` and `run` into a single hierarchy string.
/// `tag` resolves through `tags.sqlite` BOUND-only — PENDING /
/// ABSENT raise structured errors. When neither is given, falls
/// back to the cli's own `Config.agent_instance_hierarchy`.
pub(crate) async fn resolve_scope(
    ctx: &Context,
    agent_instance_hierarchy: Option<String>,
    tag: Option<String>,
) -> Result<String, Error> {
    match (agent_instance_hierarchy, tag) {
        (Some(h), None) => Ok(h),
        (None, Some(tag)) => {
            use crate::filesystem::db::tags;
            match tags::lookup_async(ctx.filesystem.clone(), tag.clone()).await? {
                tags::LookupState::Bound { agent_instance_hierarchy } => {
                    Ok(agent_instance_hierarchy)
                }
                tags::LookupState::Pending {
                    parent_agent_instance_hierarchy,
                    agent_full_id,
                } => Err(Error::TagPending {
                    tag,
                    parent_agent_instance_hierarchy,
                    agent_full_id,
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
            // `list::execute` returns the rows up-front; emit one
            // stream item per row.
            let items = list::execute(ctx, req).await?;
            Box::pin(stream::iter(
                items.into_iter().map(|r| Ok(ResponseItem::List(r))),
            ))
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
