//! `agents instances` — CLI-side dispatch for the surviving
//! instances subtree. Two leaves: `get`, `list`. `spawn`, `message`,
//! and `read` moved up to `agents spawn`, `agents message`, and
//! `agents logs read`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::instances::list::Target;
use objectiveai_sdk::cli::command::agents::instances::{Request, ResponseItem};

use crate::context::Context;
use crate::db::tags;
use crate::error::Error;

pub mod get;
pub mod list;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

/// Resolve a `Target` to the AIH it designates. `Me`/`Direct` always
/// resolve; a `Tag` resolves only when BOUND — GROUPED/ABSENT yield
/// `None` (silently skipped by both `list` and `get`). Shared by the
/// `list` and `get` handlers.
async fn resolve_target(
    db: &crate::db::Pool,
    target: Target,
    default_parent: &str,
) -> Result<Option<String>, Error> {
    match target {
        Target::Me => Ok(Some(default_parent.to_string())),
        Target::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| default_parent.to_string());
            Ok(Some(format!("{parent}/{agent_instance}")))
        }
        Target::Tag { agent_tag } => match tags::lookup(db, &agent_tag).await? {
            tags::LookupState::Bound {
                agent_instance_hierarchy,
            } => Ok(Some(agent_instance_hierarchy)),
            tags::LookupState::Grouped { .. } | tags::LookupState::Absent => Ok(None),
        },
    }
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Get(req) => {
            let inner = get::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Get)))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::GetRequestSchema(value)))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::GetResponseSchema(value)))
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
