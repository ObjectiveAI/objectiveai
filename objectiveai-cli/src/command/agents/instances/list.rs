//! `agents instances list` — enumerate the DIRECT children of one or
//! more resolved targets, with per-agent aggregates (tags, queued
//! count, spawn/active timestamps, total logged messages). Backed by
//! `db::instances::list_under_parent`.

use std::collections::HashSet;
use std::pin::Pin;

use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::instances::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();
    let db = ctx.db_client().await?.clone();
    let stream = async_stream::stream! {
        // Resolve + query every target concurrently. GROUPED/ABSENT tags
        // resolve to None and contribute nothing.
        let mut inflight = FuturesUnordered::new();
        for target in request.targets {
            let db = db.clone();
            let default_parent = default_parent.clone();
            inflight.push(async move {
                let parent = super::resolve_target(&db, target, &default_parent).await?;
                let items = match parent {
                    Some(p) => crate::db::instances::list_under_parent(&db, &p)
                        .await
                        .map_err(Error::from)?,
                    None => Vec::new(),
                };
                Ok::<Vec<ResponseItem>, Error>(items)
            });
        }
        // Yield each child the moment its target's query lands — never
        // waiting for the slowest. A `seen` set dedups by AIH across
        // overlapping/nested targets (the BTreeMap's old job, minus the
        // global sort that collecting would have forced).
        let mut seen = HashSet::new();
        while let Some(result) = inflight.next().await {
            match result {
                Ok(items) => {
                    for item in items {
                        if seen.insert(item.agent_instance_hierarchy.clone()) {
                            yield Ok(item);
                        }
                    }
                }
                Err(e) => yield Err(e),
            }
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::list as sdk;
    use objectiveai_sdk::cli::command::agents::instances::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::list as sdk;
    use objectiveai_sdk::cli::command::agents::instances::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
