//! `agents queue read pending` — stream every pending queue row
//! for the resolved targets, parts-grouped. Symmetric with
//! `agents logs read all`'s handler: `Vec<Target>` loop, per-target
//! resolution, `FuturesUnordered` fan-out.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use objectiveai_sdk::cli::command::agents::queue::read::pending::{
    Request, ResponseItem, Target,
};

use crate::context::Context;
use crate::db::message_queue::ResolvedTarget;
use crate::db::tags;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();
    let db = ctx.db_client().await?.clone();
    let after_id = request.after_id;
    let limit = request.limit;
    let stream = async_stream::stream! {
        let mut inflight = FuturesUnordered::new();
        for target in request.targets {
            let db = db.clone();
            let default_parent = default_parent.clone();
            inflight.push(async move {
                let resolved = resolve_target(&db, target, &default_parent).await?;
                let items = crate::db::message_queue::list_pending_for_targets(
                    &db, std::slice::from_ref(&resolved), after_id, limit,
                ).await.map_err(Error::from)?;
                Ok::<Vec<ResponseItem>, Error>(items)
            });
        }
        while let Some(result) = inflight.next().await {
            match result {
                Ok(items) => {
                    for item in items {
                        yield Ok(item);
                    }
                }
                Err(e) => yield Err(e),
            }
        }
    };
    Ok(Box::pin(stream))
}

/// Direct mode resolves to an AIH; tag mode resolves the tag via
/// `tags::lookup` to the tag NAME for any registered tag (BOUND or
/// GROUPED) — the queue is keyed by tag name, so it's readable even
/// before the agent is spawned. Only an unregistered (ABSENT) tag
/// errors. (Unlike `agents logs read all`, which needs the agent
/// spawned because logs don't exist until then.)
async fn resolve_target(
    db: &crate::db::Pool,
    target: Target,
    default_parent: &str,
) -> Result<ResolvedTarget, Error> {
    match target {
        Target::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent =
                parent_agent_instance_hierarchy.unwrap_or_else(|| default_parent.to_string());
            Ok(ResolvedTarget::Hierarchy(format!("{parent}/{agent_instance}")))
        }
        // The caller's own AIH itself — no child leaf appended.
        Target::Me => Ok(ResolvedTarget::Hierarchy(default_parent.to_string())),
        // A registered tag's queue is keyed by the tag NAME — queue rows
        // carry the literal tag, not the resolved hierarchy. So the queue
        // doesn't depend on the agent being spawned: a GROUPED (registered
        // but not-yet-spawned) tag has a perfectly readable queue, and a
        // rebind doesn't strand queued prompts. Both BOUND and GROUPED
        // therefore resolve to the tag name; only an unregistered (ABSENT)
        // tag has no queue.
        Target::Tag { agent_tag } => match tags::lookup(db, &agent_tag).await? {
            tags::LookupState::Bound { .. } | tags::LookupState::Grouped { .. } => {
                Ok(ResolvedTarget::Tag(agent_tag))
            }
            tags::LookupState::Absent => Err(Error::TagNotFound(agent_tag)),
        },
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::queue::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::queue::read::pending::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::queue::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::queue::read::pending::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
